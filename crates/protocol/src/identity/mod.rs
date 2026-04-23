mod peer_id;

use crate::{SignalingPayload, deserialize_base64_keypair, serialize_base64_keypair};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Biscuit, KeyPair, PublicKey};
pub use peer_id::PeerID;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
pub struct Identity {
    #[serde(
        serialize_with = "serialize_base64_keypair",
        deserialize_with = "deserialize_base64_keypair"
    )]
    keypair: KeyPair,
    known_peers: HashSet<PeerID>,
}

impl Identity {
    pub fn new() -> Self {
        Self {
            keypair: KeyPair::new(),
            known_peers: HashSet::new(),
        }
    }

    pub fn pubkey(&self) -> PublicKey {
        self.keypair.public()
    }

    pub fn create_token(&self) -> anyhow::Result<Biscuit> {
        let token = Biscuit::builder().build(&self.keypair)?;
        Ok(token)
    }

    pub fn verify(
        &self,
        payload: &SignalingPayload,
        expected_sender: &PeerID,
    ) -> anyhow::Result<()> {
        let derived_id = BASE64_URL_SAFE_NO_PAD.encode(payload.pubkey.to_bytes());
        anyhow::ensure!(
            derived_id == expected_sender.as_str(),
            "pubkey does not match sender PeerID"
        );
        Biscuit::from(&payload.token, &payload.pubkey)?;
        Ok(())
    }
    pub fn add_known_peer(&mut self, peer: PeerID) {
        self.known_peers.insert(peer);
    }
}
