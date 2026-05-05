mod peer_id;

use crate::{deserialize_base64_keypair, serialize_base64_keypair};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Biscuit, KeyPair, PublicKey};
pub use peer_id::PeerID;
use serde::{Deserialize, Serialize};

/// Persistent ED25519 identity of a peer.
///
/// The public half is the [`PeerID`] used to address the peer in the mesh.
#[derive(Serialize, Deserialize)]
pub struct Identity {
    #[serde(
        serialize_with = "serialize_base64_keypair",
        deserialize_with = "deserialize_base64_keypair"
    )]
    keypair: KeyPair,
}

impl Default for Identity {
    fn default() -> Self {
        Self::new()
    }
}

impl Identity {
    pub fn new() -> Self {
        Self {
            keypair: KeyPair::new(),
        }
    }

    pub fn pubkey(&self) -> PublicKey {
        self.keypair.public()
    }

    pub fn peer_id(&self) -> PeerID {
        PeerID(self.keypair.public())
    }

    pub fn create_token(&self, sdp: &str) -> Result<Vec<u8>> {
        let sdp_b64 = BASE64_URL_SAFE_NO_PAD.encode(sdp);
        Ok(Biscuit::builder()
            .fact(format!("sdp(\"{sdp_b64}\")").as_str())?
            .build(&self.keypair)?
            .to_vec()?)
    }
}
