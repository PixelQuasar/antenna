use serde::{Deserialize, Serialize};

/// Events fed into the handshake FSM from the driver.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeInput {
    /// Input event signalizing that we started SDP negotiation
    InitNegotiation,

    ///
    SDPOfferCreated { sdp: String },

    ///
    SDPAnswerCreated { sdp: String },

    /// Input event containing sdp offer from other peer (from host)
    SDPOfferReceived { sdp: String },

    /// Input event containing sdp answer from other peer (from joiner)
    SDPAnswerReceived { sdp: String },

    /// Input event signalizing about opening of data channel with peer
    DataChannelOpen,

    /// Input event signalizing about connection end
    Disconnected,
}
