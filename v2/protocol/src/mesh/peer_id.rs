use std::fmt;

/// Mesh peer identifier wrapper
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PeerID(String);

impl PeerID {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for PeerID {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for PeerID {
    fn from(s: String) -> Self {
        Self(s)
    }
}
