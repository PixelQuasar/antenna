use antenna_client_shared::IdentityStorage;
use antenna_protocol::Identity;
use anyhow::{Context, Result, anyhow};

use crate::STORAGE_IDENTITY_KEY;

pub struct Storage;

impl IdentityStorage for Storage {
    fn load_identity() -> Option<Identity> {
        let storage = web_sys::window()?
            .local_storage()
            .ok()??
            .get_item(STORAGE_IDENTITY_KEY)
            .ok()??;
        serde_json::from_str(&storage).ok()
    }

    fn save_identity(identity: &Identity) -> Result<()> {
        let json = serde_json::to_string(identity)?;
        web_sys::window()
            .context("DOM window is unavailable")?
            .local_storage()
            .map_err(|e| anyhow!("{:?}", e))?
            .context("LocalStorage is unavailable")?
            .set_item(STORAGE_IDENTITY_KEY, &json)
            .map_err(|e| anyhow!("{:?}", e))?;
        Ok(())
    }
}
