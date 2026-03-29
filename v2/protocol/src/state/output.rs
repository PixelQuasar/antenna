use crate::{mesh::PeerID, state::RelayPayload, transport::TransportOutput};

/// Common event that client FSM sends
#[derive(Debug, PartialEq, Eq)]
pub enum Output<T> {
    /// Transport event
    Transport {
        event: TransportOutput,
        peer: PeerID,
    },

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

    ///
    Relay { via: PeerID, payload: RelayPayload },
}
