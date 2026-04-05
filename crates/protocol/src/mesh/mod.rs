mod handle_handshake;
mod peer_id;
#[cfg(test)]
mod test;

use std::collections::{HashMap, HashSet};

pub use peer_id::PeerID;

use crate::{HandshakeContext, Input, Output, UserMsgPayload};

///
#[derive(Default, Clone)]
pub struct MeshMetadata {
    ///
    pub sdp_offer: Option<String>,

    ///
    pub sdp_answer: Option<String>,
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

    pub fn process<Msg: UserMsgPayload>(&mut self, input: Input<Msg>) -> Vec<Output<Msg>> {
        match input {
            Input::Handshake { from, event } => self.handle_handshake(from, event),
            Input::PeerLeaving { peer } => self.handle_peer_leaving(peer),
            Input::MessageReceived { peer_from, data } => {
                if self.connected.contains(&peer_from) {
                    vec![Output::ReceiveMessage { peer_from, data }]
                } else {
                    vec![]
                }
            }
            Input::Send { peer_to, data } => {
                if self.connected.contains(&peer_to) {
                    vec![Output::SendMessage {
                        peer_to,
                        data: data,
                    }]
                } else {
                    vec![]
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
                out
            }
        }
    }

    fn handle_peer_leaving<Msg: UserMsgPayload>(&mut self, peer: PeerID) -> Vec<Output<Msg>> {
        self.handshakes.remove(&peer);
        let was_connected = self.connected.remove(&peer);

        let mut out = Vec::new();
        if was_connected {
            out.push(Output::PeerDisconnected { peer });
        }
        out
    }

    pub fn metadata(&self) -> &MeshMetadata {
        &self.metadata
    }
}
