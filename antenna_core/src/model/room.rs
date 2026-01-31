use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomId(pub Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
