pub mod input;
pub mod output;

/// Transport-level state of the client transport FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    /// Client created but SDP negotiation hasn't started yet
    Idle,

    /// Host sent SDP offer to joiner and is waiting for other peer answer
    WaitingForAnswer,

    /// Joiner sent SDP answer and waiting to establish data channel with host
    WaitingForDataChannel,

    /// Client has established connection with other peer
    Connected,

    /// Connection is closed
    Closed,
}
