use crate::{
    deserialize_base64_pubkey, deserialize_base64_vec, serialize_base64_pubkey,
    serialize_base64_vec,
};
use anyhow::{Result, anyhow, ensure};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Biscuit, PublicKey, builder::AuthorizerBuilder, datalog::RunLimits};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::PeerID;

/// SDP offer or answer signed by the sender's [`crate::Identity`].
///
/// The wire form is a base64 string carrying the sender's public key plus
/// a biscuit token whose verified `sdp` fact contains the actual SDP. This
/// is what `Peer::start` / `receive_offer` / `receive_answer` produce and
/// consume.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct SignalingPayload {
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
    pub fn from_base64(data: &str) -> Result<Self> {
        let decoded = BASE64_URL_SAFE_NO_PAD.decode(data)?;
        serde_json::from_slice(&decoded).map_err(|e| anyhow!(e))
    }

    pub fn to_base64(&self) -> Result<String> {
        let json = serde_json::to_string(&self).map_err(|e| anyhow!(e))?;
        Ok(BASE64_URL_SAFE_NO_PAD.encode(json))
    }

    pub fn peer_id(&self) -> PeerID {
        PeerID(self.pubkey)
    }

    /// Verify the payload was signed by `expected_sender` and return the SDP.
    pub fn get_sdp_verified(&self, expected_sender: &PeerID) -> Result<String> {
        ensure!(
            self.pubkey == expected_sender.pubkey(),
            "pubkey does not match sender PeerID"
        );
        let token = Biscuit::from(&self.token, self.pubkey)?;
        let mut authorizer = AuthorizerBuilder::new()
            .policy("allow if true")?
            .set_limits(RunLimits {
                max_time: Duration::from_millis(100),
                ..Default::default()
            })
            .build(&token)?;
        let (sdp_b64,): (String,) = authorizer.query_exactly_one("data($s) <- sdp($s)")?;
        let bytes = BASE64_URL_SAFE_NO_PAD
            .decode(sdp_b64)
            .map_err(|e| anyhow!("invalid base64 in token sdp fact: {e}"))?;
        String::from_utf8(bytes).map_err(|e| anyhow!("invalid utf-8 in token sdp fact: {e}"))
    }
}
