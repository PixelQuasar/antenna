use antenna_protocol::Identity;
use anyhow::Result;

/// Persistence contract for the local node's identity. Each platform implements this with
/// its own backend (web → localStorage, native → JSON file). The contract is the same:
/// load may return `None` if no identity has been saved yet (or the persisted blob is
/// malformed); save must overwrite atomically.
pub trait IdentityStorage {
    fn load_identity(&self) -> Option<Identity>;
    fn save_identity(&self, identity: &Identity) -> Result<()>;
}
