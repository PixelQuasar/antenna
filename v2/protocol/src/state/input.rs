use crate::transport::TransportInput;

/// Common event that client FSM receives
#[derive(Debug, PartialEq, Eq)]
pub enum Input<T> {
    /// Transport event
    Transport(TransportInput),

    /// Receive abstract message from other peer
    MessageReceived { peer_from: u64, data: T },

    /// Send abstract message to other peer
    PeerSend { peer_to: u64, data: T },

    /// Send abstract message to all peers
    PeerBroadcast { data: T },
}
