use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct PeerId(pub Uuid);

impl PeerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(Uuid::parse_str(s).unwrap())
    }
}

impl From<String> for PeerId {
    fn from(s: String) -> Self {
        Self(Uuid::parse_str(&s).unwrap())
    }
}
