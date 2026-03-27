/// Transport-level state of the client transport FSM.
#[derive(Debug, PartialEq, Eq)]
pub enum TransportState {
    /// Client created but SDP negotiation hasn't started yet
    Idle,

    ///
    CreatingOffer,

    /// Host sent SDP offer to joiner and is waiting for other peer answer
    WaitingForAnswer { local_sdp: String },

    ///
    CreatingAnswer,

    /// Joiner sent SDP answer and waiting to establish data channel with host
    WaitingForDataChannel { local_sdp: Option<String> },

    /// Client has established connection with other peer
    Connected,

    /// Connection is closed
    Closed,
}
