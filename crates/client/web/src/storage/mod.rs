use antenna_client_shared::IdentityStorage;
use antenna_protocol::Identity;
use anyhow::{Context, Result, anyhow};

/// Identity persistence backed by `window.localStorage`.
pub struct Storage {
    path: String,
}

impl Storage {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl IdentityStorage for Storage {
    fn load_identity(&self) -> Option<Identity> {
        let raw = web_sys::window()?
            .local_storage()
            .ok()??
            .get_item(&self.path)
            .ok()??;
        serde_json::from_str(&raw).ok()
    }

    fn save_identity(&self, identity: &Identity) -> Result<()> {
        let json = serde_json::to_string(identity)?;
        web_sys::window()
            .context("DOM window is unavailable")?
            .local_storage()
            .map_err(|e| anyhow!("{:?}", e))?
            .context("LocalStorage is unavailable")?
            .set_item(&self.path, &json)
            .map_err(|e| anyhow!("{:?}", e))?;
        Ok(())
    }
}
