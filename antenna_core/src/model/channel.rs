use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Channel {
    Reliable,
    Unreliable,
    ReliableUnordered,
}

impl Default for Channel {
    fn default() -> Self {
        Self::Reliable
    }
}
