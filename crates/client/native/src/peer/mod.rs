use antenna_client_shared::{Event, IceServerConfig, RtcCallbacks};
use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{Driver, Storage};

/// Native concurrent peer implementation
pub struct Peer<Msg: UserMsgPayload + Send + Sync + 'static> {
    driver: Arc<Mutex<Driver<Msg>>>,
    callbacks: Arc<Mutex<RtcCallbacks<Msg>>>,
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
        Self { driver, callbacks }
    }

    pub async fn my_id(&self) -> PeerID {
        *self.driver.lock().await.id()
    }

    pub async fn subscribe(&self, subscription: Event<Msg>) -> u64 {
        self.callbacks.lock().await.subscribe(subscription)
    }

    pub async fn unsubscribe(&self, id: u64) -> bool {
        self.callbacks.lock().await.unsubscribe(id)
    }

    pub async fn start(&self) -> Result<String> {
        Driver::start(self.driver.clone()).await
    }

    pub async fn receive_offer(&self, offer: &str) -> Result<String> {
        Driver::receive_offer(self.driver.clone(), offer).await
    }

    pub async fn receive_answer(&self, answer: &str) -> Result<()> {
        Driver::receive_answer(self.driver.clone(), answer).await
    }

    pub fn send(&self, peer_id: PeerID, data: Msg) {
        Driver::send(self.driver.clone(), peer_id, data);
    }

    pub fn broadcast(&self, data: Msg) {
        Driver::broadcast(self.driver.clone(), data);
    }

    pub fn leave(&self) {
        Driver::leave(self.driver.clone());
    }

    pub async fn is_connected(&self, peer_id: &PeerID) -> bool {
        self.driver.lock().await.is_connected(peer_id)
    }

    pub async fn connected_peers(&self) -> HashSet<PeerID> {
        self.driver.lock().await.connected_peers()
    }

    /// Force a connection drop to `peer_id`, used in tests
    pub async fn force_drop(&self, peer_id: PeerID) -> Result<()> {
        Driver::force_drop(self.driver.clone(), peer_id).await
    }
}
