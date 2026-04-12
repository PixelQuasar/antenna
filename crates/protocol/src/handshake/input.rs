use serde::{Deserialize, Serialize};

use crate::SignalingPayload;

/// Events fed into the handshake FSM from the driver.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeInput {
    ///
    Init,

    ///
    Signaling(SignalingPayload),

    /// Driver sends when it creates SDP offer/answer
    SignalingCreated(SignalingPayload),

    /// Input event signalizing about opening of data channel with peer
    DataChannelOpen,

    /// Input event signalizing about connection end
    Disconnected,
}
