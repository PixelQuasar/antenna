mod handle_relay;
mod handle_transport;
mod peer_id;
mod test;

pub use peer_id::PeerID;

use crate::state::{Input, Output};
use crate::transport::TransportContext;
use std::collections::{HashMap, HashSet};

/// Core FSM of antenna client, handles SDP negotiation handshakes (but not signaling!!)
/// and abstract mesh logic
pub struct MeshFSM {
    // ID of current peer, must be globally unique
    id: PeerID,

    // Map of transport automati, contains state of current handshakes with other sessions
    handshakes: HashMap<PeerID, TransportContext>,

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
            Input::RelayReceived { from, payload } => self.handle_relay(from, payload),
        }
    }

    fn handle_peer_leaving<Msg>(&mut self, peer: PeerID) -> Vec<Output<Msg>> {
        self.handshakes.remove(&peer);
        let was_connected = self.connected.remove(&peer);

        let to_remove: Vec<_> = self
            .handshakes
            .iter()
            .filter_map(|(id, ctx)| {
                if ctx.via.as_ref() == Some(&peer) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut out = Vec::new();
        for id in to_remove {
            self.handshakes.remove(&id);
            out.push(Output::PeerDisconnected { peer: id });
        }

        if was_connected {
            out.push(Output::PeerDisconnected { peer });
        }
        out
    }
}
