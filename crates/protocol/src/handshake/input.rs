use serde::{Deserialize, Serialize};

use crate::SignalingPayload;

/// Events fed into the handshake FSM from the driver.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeInput {
    /// Handshake init
    Init,

    /// Local offer is created
    OfferCreated(String),

    /// Local answer is created
    AnswerCreated(String),

    /// Receiving offer from remote peer
    Offer(SignalingPayload),

    /// Receiving answer from remote peer
    Answer(SignalingPayload),

    /// Input event signalizing about opening of data channel with peer
    DataChannelOpen,

    /// Transport terminated unexpectedly.
    ConnectionDropped,
}
