use serde::{Deserialize, Serialize};

use crate::SignalingPayload;

/// Commands emitted by the handshake FSM for the driver to execute.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum HandshakeOutput {
    /// Initiate creating, applying and sending offer to other peer (to joiner)
    InitSDPOffer,

    /// Initiate creating, applying and sending answer to other peer (to host)
    RequestSDPAnswer(SignalingPayload),

    /// Apply received answer from other peer (from joiner)
    AcceptSDPAnswer(SignalingPayload),

    /// Register connect with other peer
    Connected,

    /// Close connection
    Close,
}
