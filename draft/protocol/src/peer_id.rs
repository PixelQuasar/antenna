use serde::{Deserialize, Serialize};
use std::fmt;

/// Opaque peer identifier.
///
/// For the master this is always `"master"`. For joiners it is assigned
/// by the master upon connection.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn master() -> Self {
        Self("master".to_owned())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for PeerId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}
