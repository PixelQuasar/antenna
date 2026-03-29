/// Transport-level state of the client transport FSM.
#[derive(Debug, PartialEq, Eq)]
pub enum TransportState {
    /// Client created but SDP negotiation hasn't started yet
    Idle,

    /// Host is creating SDP offer
    CreatingOffer,

    /// Host sent SDP offer to joiner and is waiting for other peer answer
    WaitingForAnswer,

    /// Joiner is creating SDP answer
    CreatingAnswer,

    /// Joiner sent SDP answer and waiting to establish data channel with host
    WaitingForDataChannel,

    /// Client has established connection with other peer
    Connected,

    /// Connection is closed
    Closed,
}
