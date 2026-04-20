mod peer_id;

use std::collections::HashSet;

pub use peer_id::PeerID;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use biscuit_auth::{
    AuthorizerBuilder, Biscuit, KeyPair, PublicKey, builder::fact, builder::string,
};

use crate::SignalingPayload;

pub struct Identity {
    keypair: KeyPair,
    id: PeerID,
    known_peers: HashSet<PeerID>,
}

impl Identity {
    pub fn new() -> Self {
        let keypair = KeyPair::new();
        let pubkey = keypair.public().clone();
        Self {
            keypair,
            id: PeerID::new(URL_SAFE_NO_PAD.encode(pubkey.to_bytes())),
            known_peers: HashSet::new(),
        }
    }

    pub fn id(&self) -> &PeerID {
        &self.id
    }

    pub fn pubkey(&self) -> PublicKey {
        self.keypair.public()
    }

    pub fn create_token(&self, for_peer: &PeerID) -> Result<Biscuit> {
        let token = Biscuit::builder()
            .fact(fact("for_peer", &[string(for_peer.as_str())]))?
            .build(&self.keypair)?;
        Ok(token)
    }

    pub fn verify(&self, payload: &SignalingPayload, expected_sender: &PeerID) -> Result<()> {
        let derived_id = URL_SAFE_NO_PAD.encode(payload.pubkey.to_bytes());
        anyhow::ensure!(
            derived_id == expected_sender.as_str(),
            "pubkey does not match sender PeerID"
        );

        let token = Biscuit::from(&payload.token, &payload.pubkey)?;

        AuthorizerBuilder::new()
            .fact(fact("my_peer_id", &[string(self.id.as_str())]))?
            .policy("allow if for_peer($p), my_peer_id($p)")?
            .build(&token)?
            .authorize()?;

        Ok(())
    }
    pub fn add_known_peer(&mut self, peer: PeerID) {
        self.known_peers.insert(peer);
    }
}
