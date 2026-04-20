/// Handshake-level state of the client handshake FSM.
#[derive(Debug, PartialEq, Eq)]
pub enum HandshakeState {
    /// Peer created but negotiation hasn't started yet
    Idle,

    /// Host is creating offer
    CreatingOffer,

    /// Host sent offer to joiner and is waiting for other peer answer
    WaitingForAnswer,

    /// Joiner is creating answer
    CreatingAnswer,

    /// Joiner sent answer and waiting to establish data channel with host
    WaitingForDataChannel,

    /// Peer has established connection with other peer
    Connected,

    /// Connection is closed
    Closed,
}
