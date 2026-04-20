use crate::{
    deserialize_base64_pubkey, deserialize_base64_vec, serialize_base64_pubkey,
    serialize_base64_vec,
};
use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::PublicKey;
use serde::{Deserialize, Serialize};

use crate::PeerID;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct SignalingPayload {
    pub sdp: String,
    #[serde(
        serialize_with = "serialize_base64_pubkey",
        deserialize_with = "deserialize_base64_pubkey"
    )]
    pub pubkey: PublicKey,
    #[serde(
        serialize_with = "serialize_base64_vec",
        deserialize_with = "deserialize_base64_vec"
    )]
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
