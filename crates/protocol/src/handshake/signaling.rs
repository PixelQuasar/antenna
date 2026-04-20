use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Algorithm, PublicKey};
use serde::{Deserialize, ser::SerializeStruct};

use crate::PeerID;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct SignalingPayload {
    pub sdp: String,
    #[serde(deserialize_with = "deserialize_base64_pubkey")]
    pub pubkey: PublicKey,
    #[serde(deserialize_with = "deserialize_base64_vec")]
    pub token: Vec<u8>,
}

impl SignalingPayload {
    pub fn from_base64(data: &str) -> anyhow::Result<Self> {
        let decoded = BASE64_URL_SAFE_NO_PAD.decode(data)?;
        serde_json::from_slice(&decoded).map_err(|e| anyhow!(e))
    }

    pub fn to_base64(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string(&self).map_err(|e| anyhow!(e))?;
        Ok(BASE64_URL_SAFE_NO_PAD.encode(json))
    }

    pub fn peer_id(&self) -> PeerID {
        PeerID::new(BASE64_URL_SAFE_NO_PAD.encode(self.pubkey.to_bytes()))
    }
}

impl serde::ser::Serialize for SignalingPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("SignalingPayload", 3)?;
        s.serialize_field("sdp", &self.sdp)?;
        s.serialize_field(
            "pubkey",
            &BASE64_URL_SAFE_NO_PAD.encode(&self.pubkey.to_bytes()),
        )?;
        s.serialize_field("token", &BASE64_URL_SAFE_NO_PAD.encode(&self.token))?;
        s.end()
    }
}

fn deserialize_base64_vec<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    BASE64_URL_SAFE_NO_PAD
        .decode(s)
        .map_err(serde::de::Error::custom)
}

fn deserialize_base64_pubkey<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    let bytes = BASE64_URL_SAFE_NO_PAD
        .decode(s)
        .map_err(serde::de::Error::custom)?;

    PublicKey::from_bytes(&bytes, Algorithm::Ed25519).map_err(serde::de::Error::custom)
}
