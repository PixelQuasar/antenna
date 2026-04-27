// Platform backend is selected by both the `web`/`native` feature flag AND the build
// target. Cargo's feature unification (when multiple workspace consumers ask for
// different features) is harmless because only the cfg branch matching the current
// target activates — so `cargo build --workspace` works even if some crates pick
// `web` and others pick `native`.

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
