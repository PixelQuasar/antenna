use crate::{handshake::HandshakeOutput, mesh::PeerID, state::RelayPayload};

/// Common event that client FSM sends
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Output<T> {
    /// Handshake event
    Handshake {
        event: HandshakeOutput,
        peer: PeerID,
    },

    /// Relay event
    Relay { via: PeerID, payload: RelayPayload },

    /// Send message to other peer in mesh
    SendMessage { peer_to: PeerID, data: T },

    /// Broadcast message to all peers in mesh
    Broadcast { data: T },

    /// Initiate receiving message from any outer sender
    ReceiveMessage { peer_from: PeerID, data: T },

    ///
    PeerConnected { peer: PeerID },

    ///
    PeerDisconnected { peer: PeerID },
}
