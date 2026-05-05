//! SDK for building decentralized P2P meshes over WebRTC.
//!
//! Antenna abstracts the low-level WebRTC API into a peer-mesh interface
//! and bundles signaling, reconnection, and identity verification. One
//! API works on both web (WASM) and native (webrtc-rs) targets.
//!
//! ## Choosing a backend
//!
//! Pick one of the platform features in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! antenna = { version = "0.1.0", features = ["web"] }            # browser runtime
//! # or
//! antenna = { version = "0.1.0", features = ["native"] }         # Tokio webrtc-rs runtime
//! ```
//!
//! The `signaling-client` feature (on by default for the underlying
//! platform crates) adds [`SignalingClient`] for the bundled signaling
//! server. Disable it if you bring your own signaling transport.
//!
//! ## Quick start (native)
//!
//! ```ignore
//! use antenna::{Peer, Storage, Event, MessageCallback};
//! use std::sync::Arc;
//!
//! # #[derive(Clone, serde::Serialize, serde::Deserialize)]
//! # struct Message(String);
//! let peer = Arc::new(Peer::<Message>::new(Storage::new("./identity.json")));
//!
//! peer.subscribe(Event::UserMessage(MessageCallback::from_fn(
//!     |peer_id, msg: &Message| {
//!         println!("[{peer_id}] {}", msg.0);
//!         Ok(())
//!     },
//! ))).await;
//!
//! let offer = peer.start().await?;          // host
//! // ...exchange `offer`/`answer` with the other side via any transport...
//! peer.broadcast(Message("hello".into()));
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! For a runnable example see [`example/shell-chat`](https://github.com/PixelQuasar/antenna/tree/main/example/shell-chat)
//! (native) or [`example/minimal-chat`](https://github.com/PixelQuasar/antenna/tree/main/example/minimal-chat) (WASM).
//!
//! Full protocol details are in [`docs/protocol.md`](https://github.com/PixelQuasar/antenna/blob/main/docs/protocol.md).

pub use antenna_client_shared::{
    Event, IceServerConfig, MessageCallback, NoArgCallback, PeerCallback,
};
pub use antenna_protocol::PeerID;

#[cfg(all(feature = "web", target_family = "wasm"))]
pub use antenna_client_web::{Peer, Storage, js_message, js_no_arg, js_peer};

#[cfg(all(feature = "web", feature = "signaling-client", target_family = "wasm"))]
pub use antenna_client_web::SignalingClient;

#[cfg(all(feature = "native", not(target_family = "wasm")))]
pub use antenna_client_native::{Peer, Storage};

#[cfg(all(
    feature = "native",
    feature = "signaling-client",
    not(target_family = "wasm")
))]
pub use antenna_client_native::SignalingClient;
