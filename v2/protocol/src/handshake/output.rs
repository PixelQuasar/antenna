/// Commands emitted by the handshake FSM for the driver to execute.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HandshakeOutput {
    /// Initiate creating, applying and sending SDP offer to other peer (to joiner)
    InitSDPOffer,

    /// Initiate creating, applying and sending SDP answer to other peer (to host)
    InitSDPAnswer { offer_sdp: String },

    /// Apply received SDP answer from other peer (from joiner)
    AcceptSDPAnswer { sdp: String },

    /// Close RTC connection
    Close,
}
