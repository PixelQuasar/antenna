use antenna_client_shared::IdentityStorage;
use antenna_protocol::Identity;
use anyhow::{Context, Result};
use std::path::Path;

/// Identity persistence backed by a single JSON file.
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
        let bytes = std::fs::read(&self.path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn save_identity(&self, identity: &Identity) -> Result<()> {
        let path = Path::new(&self.path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).context("creating storage directory")?;
        }
        let json = serde_json::to_vec_pretty(identity)?;
        std::fs::write(path, json).context("writing identity file")?;
        Ok(())
    }
}
