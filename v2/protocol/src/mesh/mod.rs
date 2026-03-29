mod peer_id;
mod test;

pub use peer_id::PeerID;

use crate::state::{Input, Output, RelayPayload};
use crate::transport::{Host, Joiner, TransportFSM, TransportInput, TransportState};
use std::collections::{HashMap, HashSet};

/// Core FSM of antenna client, handles SDP negotiation handshakes (but not signaling!!)
/// and abstract mesh logic
pub struct MeshFSM {
    // ID of current peer, must be globally unique
    id: PeerID,

    // Map of transport automati, contains state of current handshakes with other sessions
    handshakes: HashMap<PeerID, TransportFSM>,

    // Map of peers with established connection
    connected: HashSet<PeerID>,
}

impl MeshFSM {
    pub fn new(id: PeerID) -> Self {
        Self {
            id,
            handshakes: HashMap::new(),
            connected: HashSet::new(),
        }
    }

    pub fn id(&self) -> &PeerID {
        &self.id
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.connected.contains(peer)
    }

    pub fn connected_peers(&self) -> &HashSet<PeerID> {
        &self.connected
    }

    pub fn process<Msg>(&mut self, input: Input<Msg>) -> Vec<Output<Msg>> {
        match input {
            Input::Transport { peer, event } => self.handle_transport(peer, event),
            Input::PeerLeaving { peer } => self.handle_peer_leaving(peer),
            Input::MessageReceived { peer_from, data } => {
                if self.connected.contains(&peer_from) {
                    vec![Output::ReceiveMessage { peer_from, data }]
                } else {
                    vec![]
                }
            }
            Input::PeerSend { peer_to, data } => {
                if self.connected.contains(&peer_to) {
                    vec![Output::SendMessage { peer_to, data }]
                } else {
                    vec![]
                }
            }
            Input::PeerBroadcast { data } => {
                if !self.connected.is_empty() {
                    vec![Output::Broadcast { data }]
                } else {
                    vec![]
                }
            }
            Input::RelayReceived { from, payload } => match payload {
                RelayPayload::PeerJoined { peer } => {
                    if self.connected.contains(&peer) || self.handshakes.contains_key(&peer) {
                        return vec![];
                    }

                    let we_host = self.id < peer;

                    let mut transport = if we_host {
                        TransportFSM::Host(Host::new())
                    } else {
                        TransportFSM::Joiner(Joiner::new())
                    };

                    let mut out = Vec::new();

                    if we_host {
                        // We initiate — kick off the handshake
                        if let Some(event) = transport.process(TransportInput::InitNegotiation) {
                            out.push(Output::Transport {
                                peer: peer.clone(),
                                event,
                            });
                        }
                    }
                    // If Joiner — we wait for TransportForward with SDPOfferReceived

                    self.handshakes.insert(peer, transport);
                    out
                }
                RelayPayload::TransportForward { target, event } => {
                    if self.connected.contains(&target) {
                        self.handle_transport(target, event)
                    } else {
                        vec![Output::Relay {
                            via: from,
                            payload: RelayPayload::TransportForward { target, event },
                        }]
                    }
                }
                RelayPayload::PeerLeft { peer } => self.handle_peer_leaving(peer),
            },
        }
    }

    fn handle_transport<Msg>(&mut self, peer: PeerID, event: TransportInput) -> Vec<Output<Msg>> {
        if !self.handshakes.contains_key(&peer) {
            let transport = match &event {
                TransportInput::InitNegotiation => TransportFSM::Host(Host::new()),
                TransportInput::SDPOfferReceived { .. } => TransportFSM::Joiner(Joiner::new()),
                _ => return vec![],
            };
            self.handshakes.insert(peer.clone(), transport);
        }

        let transport = self.handshakes.get_mut(&peer).unwrap();

        let transport_out = transport.process(event);
        match &transport.state() {
            TransportState::Connected => {
                self.handshakes.remove(&peer);
                self.connected.insert(peer.clone());
                let mut out = vec![Output::PeerConnected { peer: peer.clone() }];
                for remote in &self.connected {
                    if remote != &peer {
                        out.push(Output::Relay {
                            via: remote.clone(),
                            payload: RelayPayload::PeerJoined { peer: peer.clone() },
                        });
                        out.push(Output::Relay {
                            via: peer.clone(),
                            payload: RelayPayload::PeerJoined {
                                peer: remote.clone(),
                            },
                        });
                    }
                }
                out
            }
            TransportState::Closed => {
                self.handshakes.remove(&peer);
                let mut out = vec![Output::PeerDisconnected { peer: peer.clone() }];
                if let Some(event) = transport_out {
                    out.push(Output::Transport { peer, event });
                }
                out
            }
            _ => transport_out
                .map(|event| vec![Output::Transport { peer, event }])
                .unwrap_or_default(),
        }
    }

    fn handle_peer_leaving<Msg>(&mut self, peer: PeerID) -> Vec<Output<Msg>> {
        self.handshakes.remove(&peer);
        let was_connected = self.connected.remove(&peer);

        if was_connected {
            vec![Output::PeerDisconnected { peer }]
        } else {
            vec![]
        }
    }
}
