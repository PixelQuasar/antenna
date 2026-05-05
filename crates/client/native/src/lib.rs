//! Native (Tokio + `webrtc-rs`) platform driver for the antenna P2P mesh protocol.
//!
//! Wraps `antenna-protocol`'s `MeshNodeFSM` and bridges it to the
//! `webrtc` crate for the WebRTC stack and `tokio-tungstenite` for
//! signaling.

mod driver;
mod peer;
#[cfg(feature = "signaling-client")]
mod signaling;
mod storage;
mod webrtc;

pub(crate) use driver::Driver;
pub use peer::Peer;
#[cfg(feature = "signaling-client")]
pub use signaling::SignalingClient;
pub use storage::Storage;
pub(crate) use webrtc::{ConnectionManager, DataChannelManager, PeerConnectionManager};
