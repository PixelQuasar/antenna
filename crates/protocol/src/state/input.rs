use crate::{handshake::HandshakeInput, mesh::PeerID};

/// Common event that client FSM receives
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input<T> {
    /// Reveice handshake event
    Handshake { event: HandshakeInput, from: PeerID },

    /// Receive abstract message
    MessageReceived { peer_from: PeerID, data: T },

    /// Send abstract message to other peer
    PeerSend { peer_to: PeerID, data: T },

    /// Send abstract message to all peers
    PeerBroadcast { data: T },

    /// Receive peer leaving message
    PeerLeaving { peer: PeerID },
}
