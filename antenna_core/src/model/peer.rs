use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerId(pub Uuid);

impl PeerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
