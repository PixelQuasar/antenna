use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;

use antenna_client_shared::IceServerConfig;

use crate::{DataChannelManager, PeerConnectionManager};

/// Combines a peer connection with its (optional) data channel and a stored on-data-channel
/// handler closure for the joiner side. Mirrors the web `ConnectionManager`.
pub struct ConnectionManager {
    pc: PeerConnectionManager,
    dc: Mutex<Option<DataChannelManager>>,
}

impl ConnectionManager {
    pub async fn new_host(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers).await?;
        let dc = DataChannelManager::new(pc.peer_connection(), "data").await?;
        Ok(Self {
            pc,
            dc: Mutex::new(Some(dc)),
        })
    }

    pub async fn new_joiner(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers).await?;
        Ok(Self {
            pc,
            dc: Mutex::new(None),
        })
    }

    pub async fn create_offer(&self) -> Result<String> {
        let sdp = self.pc.create_offer().await?;
        self.pc.set_local_description(&sdp, true).await?;
        self.pc.fetch_sdp().await
    }

    pub async fn create_answer(&self, offer_sdp: &str) -> Result<String> {
        self.pc.set_remote_description(offer_sdp, true).await?;
        let sdp = self.pc.create_answer().await?;
        self.pc.set_local_description(&sdp, false).await?;
        self.pc.fetch_sdp().await
    }

    pub async fn accept_answer(&self, answer_sdp: &str) -> Result<()> {
        self.pc.set_remote_description(answer_sdp, false).await
    }

    pub async fn set_dc(&self, dc: DataChannelManager) {
        *self.dc.lock().await = Some(dc);
    }

    pub fn dc(&self) -> &Mutex<Option<DataChannelManager>> {
        &self.dc
    }

    pub fn setup_on_ice_state_change<F>(&self, callback: F)
    where
        F: FnMut(RTCIceConnectionState) + Send + Sync + 'static,
    {
        self.pc.setup_on_ice_state_change(callback);
    }

    /// Joiner-side: install a handler invoked when the remote host's data channel arrives
    pub fn setup_on_data_channel<F>(&self, mut callback: F)
    where
        F: FnMut(Arc<RTCDataChannel>) + Send + Sync + 'static,
    {
        self.pc
            .peer_connection()
            .on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
                callback(dc);
                Box::pin(async {})
            }));
    }

    pub async fn close(&self) {
        if let Some(dc) = self.dc.lock().await.take() {
            dc.close().await;
        }
        self.pc.close().await;
    }
}
