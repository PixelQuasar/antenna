use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Algorithm, PublicKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Mesh peer identifier — wraps the peer's biscuit public key directly.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PeerID(pub PublicKey);

impl PeerID {
    pub fn new(pubkey: PublicKey) -> Self {
        Self(pubkey)
    }

    pub fn pubkey(&self) -> PublicKey {
        self.0
    }

    pub fn from_base64(s: &str) -> anyhow::Result<Self> {
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(s)?;
        PublicKey::from_bytes(&bytes, Algorithm::Ed25519)
            .map(Self)
            .map_err(|e| anyhow::anyhow!("invalid peer id pubkey: {e}"))
    }
}

impl Ord for PeerID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bytes().cmp(&other.0.to_bytes())
    }
}

impl PartialOrd for PeerID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for PeerID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state);
    }
}

impl fmt::Display for PeerID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&BASE64_URL_SAFE_NO_PAD.encode(self.0.to_bytes()))
    }
}

impl Serialize for PeerID {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(self.0.to_bytes()))
    }
}

impl<'de> Deserialize<'de> for PeerID {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let bytes = BASE64_URL_SAFE_NO_PAD
            .decode(s)
            .map_err(serde::de::Error::custom)?;
        PublicKey::from_bytes(&bytes, Algorithm::Ed25519)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}
