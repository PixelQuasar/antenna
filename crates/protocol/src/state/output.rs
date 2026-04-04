use crate::{handshake::HandshakeOutput, mesh::PeerID};

/// Common event that client FSM sends
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Output<T> {
    /// Handshake event
    Handshake {
        event: HandshakeOutput,
        peer: PeerID,
    },

    /// Send message to other peer in mesh
    SendMessage { peer_to: PeerID, data: T },

    /// Broadcast message to all peers in mesh
    Broadcast { data: T },

    /// Initiate receiving message from any outer sender
    ReceiveMessage { peer_from: PeerID, data: T },

    ///
    PeerAppeared { peer: PeerID },

    /// Notify about new peer connected to mesh
    PeerConnected { peer: PeerID },

    /// Notify about new peer disconnected to mesh
    PeerDisconnected { peer: PeerID },
}
