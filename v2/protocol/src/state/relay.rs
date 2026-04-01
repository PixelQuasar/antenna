use crate::{HandshakeInput, mesh::PeerID};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RelayPayload {
    /// Event emitting connection between two peers by relay
    ConnectionRequest { peer: PeerID },

    /// Forwarded handshake event for a peer you're not directly connected to yet
    HandshakeForward {
        src: PeerID,
        dst: PeerID,
        event: HandshakeInput,
    },
}
