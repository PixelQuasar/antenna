//! WebAssembly platform driver for the antenna P2P mesh protocol.
//!
//! Wraps `antenna-protocol`'s `MeshNodeFSM` and bridges it to the browser's
//! `RTCPeerConnection`, `RTCDataChannel`, and `WebSocket` via `web-sys`.

mod dispatcher;
mod driver;
mod peer;
#[cfg(feature = "signaling-client")]
mod signaling;
mod storage;
mod utils;
mod webrtc;

pub use dispatcher::{js_message, js_no_arg, js_peer};
pub use peer::Peer;
#[cfg(feature = "signaling-client")]
pub use signaling::SignalingClient;
pub use storage::Storage;

pub(crate) use driver::Driver;
pub(crate) use utils::JsEventCallback;
pub(crate) use webrtc::{ConnectionManager, DataChannelManager, PeerConnectionManager};
