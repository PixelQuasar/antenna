mod handle_handshake;
mod handle_message;

#[cfg(test)]
mod test;

use crate::{
    HandshakeFSM, HandshakeMode, HandshakeState, Identity, Input, Output, PeerID, SignalingPayload,
    UserMsgPayload,
};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use std::collections::{HashMap, HashSet};

///
#[derive(Default, Clone)]
pub struct MeshMetadata {
    ///
    pub offer: Option<SignalingPayload>,

    ///
    pub answer: Option<SignalingPayload>,
}

pub struct HandshakeContext {
    pub fsm: HandshakeFSM,
    pub mode: HandshakeMode,
}

/// Core FSM of antenna client, handles negotiation handshakes (but not signaling!!)
/// and abstract mesh logic
pub struct MeshNodeFSM {
    /// current peer ID
    id: PeerID,

    /// identity of current peer: id and key pair
    identity: Identity,

    /// Map of handshake automati, contains state of current handshakes with other sessions
    connections: HashMap<PeerID, HandshakeContext>,

    ///
    metadata: MeshMetadata,
}

impl MeshNodeFSM {
    pub fn new() -> Self {
        Self::with_identity(Identity::new())
    }

    pub fn with_identity(identity: Identity) -> Self {
        Self {
            id: PeerID::new(BASE64_URL_SAFE_NO_PAD.encode(identity.pubkey().to_bytes())),
            identity,
            connections: HashMap::new(),
            metadata: MeshMetadata::default(),
        }
    }

    pub fn id(&self) -> &PeerID {
        &self.id
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.connections.contains_key(peer)
            && *self.connections.get(peer).unwrap().fsm.state() == HandshakeState::Connected
    }

    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.connections
            .iter()
            .filter(|x| *x.1.fsm.state() == HandshakeState::Connected)
            .map(|x| x.0.clone())
            .collect()
    }

    pub fn connected_number(&self) -> usize {
        self.connections.iter().fold(0, |a, x| {
            if *x.1.fsm.state() == HandshakeState::Connected {
                a + 1
            } else {
                a
            }
        })
    }

    pub fn process<Msg: UserMsgPayload>(&mut self, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        match input {
            Input::InitHandshake {
                with,
                mode,
                strategy,
            } => {
                self.connections.insert(
                    with,
                    HandshakeContext {
                        fsm: HandshakeFSM::new(strategy),
                        mode,
                    },
                );
                return Ok(vec![]);
            }
            Input::Handshake { from, event } => self.handle_handshake(from, event),
            Input::PeerLeaving { peer } => self.handle_peer_leaving(peer),
            Input::MessageReceived { peer_from, data } => self.handle_message(peer_from, data),
            Input::Send { peer_to, data } => {
                if self.is_connected(&peer_to) {
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
                for (peer, _) in &self.connections {
                    if !self.is_connected(peer) {
                        continue;
                    }
                    out.push(Output::SendMessage {
                        peer_to: peer.clone(),
                        data: data.clone(),
                    })
                }
                Ok(out)
            }
        }
    }

    pub fn metadata(&self) -> &MeshMetadata {
        &self.metadata
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    fn handle_peer_leaving<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        let was_connected = self.connections.remove(&peer);

        let mut out = Vec::new();
        if was_connected.is_some() {
            out.push(Output::PeerDisconnected { peer });
        }
        Ok(out)
    }
}
