//! Platform-independent traits and constants shared between the antenna platform drivers
//! ([`antenna-client-web`](https://crates.io/crates/antenna-client-web) and
//! [`antenna-client-native`](https://crates.io/crates/antenna-client-native)).

mod dispatcher;
mod ice;
mod signaling;
mod storage;

pub use dispatcher::{
    Event, EventType, MessageCallback, NoArgCallback, PeerCallback, RtcCallbacks,
};
pub use ice::IceServerConfig;
pub use signaling::{ClientMsg, ServerMsg};
pub use storage::IdentityStorage;

/// fsm-polling method in peer recursion fuel
pub const EXECUTE_FUEL: u64 = 1024;
/// Storage identity key
pub const STORAGE_IDENTITY_KEY: &str = "antenna_identity";
