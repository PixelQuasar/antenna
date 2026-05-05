//! SansIO core of the [antenna](https://crates.io/crates/antenna) P2P mesh SDK.
//!
//! This crate is the platform-independent engine that drives every antenna
//! peer. It contains no network, no async runtime, no WebRTC — only a
//! synchronous state machine that turns [`Input`] events into [`Output`]
//! commands. Platform drivers (`antenna-client-web`, `antenna-client-native`)
//! wrap it and perform the actual I/O.
//!
//! The full protocol — handshake
//! modes, relay, reconnection, mesh-completeness guarantees — is described
//! in [`docs/protocol.md`](https://github.com/PixelQuasar/antenna/blob/main/docs/protocol.md).

mod handshake;
mod identity;
mod mesh;
mod state;
#[cfg(test)]
mod test;
mod utils;

pub use handshake::{
    HandshakeFSM, HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeState,
    HandshakeStrategy, Host, Joiner, SignalingPayload,
};
pub use identity::{Identity, PeerID};
pub use mesh::{FSMState, MeshNodeFSM};
pub use state::{Input, MsgPayload, Output, RelayPayload, Scheduled, UserMsgPayload};
pub(crate) use utils::{
    MAX_RECONNECT_ATTEMPTS, RECONNECT_INTERVAL_MS, deserialize_base64_keypair,
    deserialize_base64_pubkey, deserialize_base64_vec, serialize_base64_keypair,
    serialize_base64_pubkey, serialize_base64_vec,
};
