mod handle_handshake;
mod handle_message;
mod peer_id;
#[cfg(test)]
mod test;

use crate::{
    HandshakeFSM, HandshakeInput, HandshakeMode, HandshakeStrategy, Input, Output, UserMsgPayload,
};
use anyhow::Result;
pub use peer_id::PeerID;
use std::collections::{HashMap, HashSet};

///
#[derive(Default, Clone)]
pub struct MeshMetadata {
    ///
    pub sdp_offer: Option<String>,

    ///
    pub sdp_answer: Option<String>,
}

pub struct HandshakeContext {
    pub fsm: HandshakeFSM,
    pub relay_via: Option<PeerID>,
}

/// Core FSM of antenna client, handles SDP negotiation handshakes (but not signaling!!)
/// and abstract mesh logic
pub struct MeshNodeFSM {
    /// ID of current peer, must be globally unique
    id: PeerID,

    /// Map of handshake automati, contains state of current handshakes with other sessions
    handshakes: HashMap<PeerID, HandshakeContext>,

    /// Map of peers with established connection
    connected: HashSet<PeerID>,

    ///
    metadata: MeshMetadata,
}

impl MeshNodeFSM {
    pub fn new(id: PeerID) -> Self {
        Self {
            id,
            handshakes: HashMap::new(),
            connected: HashSet::new(),
            metadata: MeshMetadata::default(),
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

    pub fn process<Msg: UserMsgPayload>(&mut self, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        match input {
            Input::InitHandshake {
                with,
                mode,
                strategy,
            } => {
                self.handshakes.insert(
                    with,
                    HandshakeContext {
                        fsm: HandshakeFSM::new(mode.clone(), strategy),
                        relay_via: match mode {
                            HandshakeMode::Bootstrap => None,
                            HandshakeMode::Relay(via) => Some(via),
                        },
                    },
                );
                return Ok(vec![]);
            }
            Input::Handshake { from, event } => self.handle_handshake(from, event),
            Input::PeerLeaving { peer } => self.handle_peer_leaving(peer),
            Input::MessageReceived { peer_from, data } => self.handle_message(peer_from, data),
            Input::Send { peer_to, data } => {
                if self.connected.contains(&peer_to) {
                    Ok(vec![Output::SendMessage {
                        peer_to,
                        data: data,
                    }])
                } else {
                    Ok(vec![])
                }
            }
            Input::Broadcast { data } => {
                let mut out = vec![];
                for peer in &self.connected {
                    out.push(Output::SendMessage {
                        peer_to: peer.clone(),
                        data: data.clone(),
                    })
                }
                Ok(out)
            }
        }
    }

    fn handle_peer_joined<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        if peer == self.id || self.connected.contains(&peer) {
            return Ok(vec![]);
        }

        self.process::<Msg>(Input::InitHandshake {
            with: peer.clone(),
            mode: HandshakeMode::Relay(self.id.clone()),
            strategy: HandshakeStrategy::Host,
        })?;

        self.process::<Msg>(Input::Handshake {
            from: peer,
            event: HandshakeInput::StartAsHost,
        })
    }
    fn handle_peer_leaving<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        self.handshakes.remove(&peer);
        let was_connected = self.connected.remove(&peer);

        let mut out = Vec::new();
        if was_connected {
            out.push(Output::PeerDisconnected { peer });
        }
        Ok(out)
    }

    pub fn metadata(&self) -> &MeshMetadata {
        &self.metadata
    }
}
