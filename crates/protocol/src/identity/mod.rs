mod peer_id;

use crate::{SignalingPayload, deserialize_base64_keypair, serialize_base64_keypair};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{
    AuthorizerBuilder, Biscuit, KeyPair, PublicKey, builder::fact, builder::string,
};
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

    pub fn create_token(&self, for_peer: &PeerID) -> anyhow::Result<Biscuit> {
        let token = Biscuit::builder()
            .fact(fact("for_peer", &[string(for_peer.as_str())]))?
            .build(&self.keypair)?;
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

        let token = Biscuit::from(&payload.token, &payload.pubkey)?;

        AuthorizerBuilder::new()
            .fact(fact(
                "my_peer_id",
                &[string(
                    BASE64_URL_SAFE_NO_PAD
                        .encode(self.pubkey().to_bytes())
                        .as_str(),
                )],
            ))?
            .policy("allow if for_peer($p), my_peer_id($p)")?
            .build(&token)?
            .authorize()?;

        Ok(())
    }
    pub fn add_known_peer(&mut self, peer: PeerID) {
        self.known_peers.insert(peer);
    }
}
