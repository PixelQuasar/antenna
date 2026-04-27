use antenna_client_shared::{Event, IceServerConfig, RtcCallbacks};
use antenna_protocol::{
    HandshakeInput, HandshakeMode, HandshakeStrategy, Input, MsgPayload, Output, PeerID,
    SignalingPayload, UserMsgPayload,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::{Driver, Storage};

/// Native concurrent peer implementation
pub struct Peer<Msg: UserMsgPayload + Send + Sync + 'static> {
    driver: Arc<Mutex<Driver<Msg>>>,
    callbacks: Arc<Mutex<RtcCallbacks<Msg>>>,
    left: Arc<AtomicBool>,
}

impl<Msg: UserMsgPayload + Send + Sync + 'static> Peer<Msg> {
    pub fn new(storage: Storage) -> Self {
        Self::with_ice_servers(storage, IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(storage: Storage, ice_servers: Vec<IceServerConfig>) -> Self {
        let callbacks = Arc::new(Mutex::new(RtcCallbacks::new()));
        let driver = Arc::new(Mutex::new(Driver::new(
            ice_servers,
            callbacks.clone(),
            storage,
        )));
        Self {
            driver,
            callbacks,
            left: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn my_id(&self) -> PeerID {
        self.driver.lock().await.id().clone()
    }

    pub async fn subscribe(&self, subscription: Event<Msg>) -> u64 {
        self.callbacks.lock().await.subscribe(subscription)
    }

    pub async fn unsubscribe(&self, id: u64) -> bool {
        self.callbacks.lock().await.unsubscribe(id)
    }

    pub async fn start(&self) -> Result<String> {
        let outputs = Driver::execute(self.driver.clone(), Input::InitOpenOffer).await?;
        outputs
            .into_iter()
            .find_map(|o| match o {
                Output::OfferReady(payload) => Some(payload),
                _ => None,
            })
            .context("Offer not found on starting")?
            .to_base64()
    }

    pub async fn receive_offer(&self, offer: &str) -> Result<String> {
        let offer = SignalingPayload::from_base64(offer)?;
        let peer_id = offer.peer_id();
        Driver::execute(
            self.driver.clone(),
            Input::InitHandshake {
                with: peer_id.clone(),
                mode: HandshakeMode::Bootstrap,
                strategy: HandshakeStrategy::Joiner,
            },
        )
        .await?;
        let outputs = Driver::execute(
            self.driver.clone(),
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Offer(offer),
            },
        )
        .await?;
        outputs
            .into_iter()
            .find_map(|o| match o {
                Output::AnswerReady(payload) => Some(payload),
                _ => None,
            })
            .context("Answer not found on receiving offer")?
            .to_base64()
    }

    pub async fn receive_answer(&self, answer: &str) -> Result<()> {
        let answer = SignalingPayload::from_base64(answer)?;
        let peer_id = answer.peer_id();
        Driver::execute(
            self.driver.clone(),
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Answer(answer),
            },
        )
        .await?;
        Ok(())
    }

    pub fn send(&self, peer_id: PeerID, data: Msg) {
        Driver::dispatch_input(
            self.driver.clone(),
            Input::Send {
                peer_to: peer_id,
                data: MsgPayload::User(data),
            },
            "Peer::send",
        );
    }

    pub fn broadcast(&self, data: Msg) {
        Driver::dispatch_input(
            self.driver.clone(),
            Input::Broadcast {
                data: MsgPayload::User(data),
            },
            "Peer::broadcast",
        );
    }

    pub fn leave(&self) {
        if self.left.swap(true, Ordering::SeqCst) {
            return;
        }
        Driver::dispatch_input(self.driver.clone(), Input::Leave, "Peer::leave");
    }

    pub async fn is_connected(&self, peer_id: &PeerID) -> bool {
        self.driver.lock().await.is_connected(peer_id)
    }

    pub async fn connected_peers(&self) -> HashSet<PeerID> {
        self.driver.lock().await.connected_peers()
    }

    /// Force a connection drop to `peer_id`, used in tests
    pub async fn force_drop(&self, peer_id: PeerID) -> Result<()> {
        Driver::execute(
            self.driver.clone(),
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::ConnectionDropped,
            },
        )
        .await?;
        Ok(())
    }
}
