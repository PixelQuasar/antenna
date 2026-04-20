use serde::{Deserialize, Serialize};

use crate::SignalingPayload;

/// Events fed into the handshake FSM from the driver.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeInput {
    ///
    Init,

    /// Driver sends when it creates offer/answer
    OfferCreated(String),

    ///
    AnswerCreated(String),

    ///
    Offer(SignalingPayload),

    ///
    Answer(SignalingPayload),

    /// Input event signalizing about opening of data channel with peer
    DataChannelOpen,

    /// Input event signalizing about connection end
    Disconnected,
}
