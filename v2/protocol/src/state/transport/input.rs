/// Events fed into the transport FSM from the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportInput {
    /// Input event signalizing that we started SDP negotiation
    InitNegotiation,

    /// Input event containing sdp offer from other peer (from host)
    SDPOfferReceived { sdp: String },

    /// Input event containing sdp answer from other peer (from joiner)
    SDPAnswerReceived { sdp: String },

    ///  Input event signalizing about opening of data channel with peer
    DataChannelOpen,

    /// Input event signalizing about connection end
    Disconnected,
}
