use antenna_protocol::Identity;
use anyhow::Result;

pub trait IdentityStorage {
    fn load_identity() -> Option<Identity>;
    fn save_identity(identity: &Identity) -> Result<()>;
}
