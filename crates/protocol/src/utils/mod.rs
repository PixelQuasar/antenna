mod ser;

pub use ser::{
    deserialize_base64_keypair, deserialize_base64_pubkey, deserialize_base64_vec,
    serialize_base64_keypair, serialize_base64_pubkey, serialize_base64_vec,
};

/// Linear delay between reconnection attempts.
pub const RECONNECT_INTERVAL_MS: u64 = 2000;

/// Number of relay-based reconnection attempts before giving up on a lost peer
/// (and falling back to signaling-server rejoin if the local node is isolated).
pub const MAX_RECONNECT_ATTEMPTS: u32 = 5;
