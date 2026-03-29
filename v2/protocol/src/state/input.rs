use crate::{mesh::PeerID, state::RelayPayload, transport::TransportInput};

/// Common event that client FSM receives
#[derive(Debug, PartialEq, Eq)]
pub enum Input<T> {
    /// Transport event
    Transport { event: TransportInput, peer: PeerID },

    /// Receive abstract message from other peer
    MessageReceived { peer_from: PeerID, data: T },

    /// Send abstract message to other peer
    PeerSend { peer_to: PeerID, data: T },

    /// Send abstract message to all peers
    PeerBroadcast { data: T },

    ///
    PeerLeaving { peer: PeerID },

    ///
    RelayReceived { from: PeerID, payload: RelayPayload },
}
