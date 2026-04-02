use crate::{handshake::HandshakeInput, mesh::PeerID, state::RelayPayload};

/// Common event that client FSM receives
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input<T> {
    /// Reveice handshake event
    Handshake { event: HandshakeInput, from: PeerID },

    /// Receive relay event
    Relay { from: PeerID, payload: RelayPayload },

    /// Receive abstract message
    MessageReceived { peer_from: PeerID, data: T },

    /// Send abstract message to other peer
    PeerSend { peer_to: PeerID, data: T },

    /// Send abstract message to all peers
    PeerBroadcast { data: T },

    /// Receive peer leaving message
    PeerLeaving { peer: PeerID },
}
