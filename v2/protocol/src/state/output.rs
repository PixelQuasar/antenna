use crate::transport::TransportOutput;

/// Common event that client FSM sends
#[derive(Debug, PartialEq, Eq)]
pub enum Output<T> {
    /// Transport event
    Transport(TransportOutput),

    /// Send message to other peer in mesh
    SendMessage { peer_to: u64, data: T },

    /// Broadcast message to all peers in mesh
    Broadcast { data: T },

    /// Initiate receiving message from any outer sender
    ReceiveMessage { peer_from: u64, data: T },
}
