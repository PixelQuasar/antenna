use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use antenna_client_shared::IceServerConfig;

/// RTCPeerConnection wrapper
pub struct PeerConnectionManager {
    pc: Arc<RTCPeerConnection>,
}

impl PeerConnectionManager {
    pub async fn from_ice_config(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let api = APIBuilder::new().build();
        let cfg = RTCConfiguration {
            ice_servers: ice_servers.iter().map(to_rtc_ice).collect(),
            ..Default::default()
        };
        let pc = api
            .new_peer_connection(cfg)
            .await
            .map_err(|e| anyhow!("new_peer_connection failed: {e}"))?;
        Ok(Self { pc: Arc::new(pc) })
    }

    pub fn peer_connection(&self) -> &Arc<RTCPeerConnection> {
        &self.pc
    }

    pub async fn create_offer(&self) -> Result<String> {
        let offer = self
            .pc
            .create_offer(None)
            .await
            .map_err(|e| anyhow!("create_offer failed: {e}"))?;
        Ok(offer.sdp)
    }

    pub async fn create_answer(&self) -> Result<String> {
        let answer = self
            .pc
            .create_answer(None)
            .await
            .map_err(|e| anyhow!("create_answer failed: {e}"))?;
        Ok(answer.sdp)
    }

    pub async fn set_local_description(&self, sdp: &str, is_offer: bool) -> Result<()> {
        let desc = build_session_description(sdp, is_offer)?;
        self.pc
            .set_local_description(desc)
            .await
            .map_err(|e| anyhow!("set_local_description failed: {e}"))?;
        Ok(())
    }

    pub async fn set_remote_description(&self, sdp: &str, is_offer: bool) -> Result<()> {
        let desc = build_session_description(sdp, is_offer)?;
        self.pc
            .set_remote_description(desc)
            .await
            .map_err(|e| anyhow!("set_remote_description failed: {e}"))?;
        Ok(())
    }

    /// Wait until ICE candidate gathering finishes, then return the final local SDP
    pub async fn fetch_sdp(&self) -> Result<String> {
        let mut gather_complete = self.pc.gathering_complete_promise().await;
        let _ = gather_complete.recv().await;
        let desc = self
            .pc
            .local_description()
            .await
            .context("no local description after ICE gathering")?;
        Ok(desc.sdp)
    }

    /// Wires the underlying webrtc-rs handler
    pub fn setup_on_ice_state_change<F>(&self, mut callback: F)
    where
        F: FnMut(RTCIceConnectionState) + Send + Sync + 'static,
    {
        self.pc
            .on_ice_connection_state_change(Box::new(move |state| {
                callback(state);
                Box::pin(async {})
            }));
    }

    pub async fn close(&self) {
        let _ = self.pc.close().await;
    }
}

fn build_session_description(sdp: &str, is_offer: bool) -> Result<RTCSessionDescription> {
    if is_offer {
        Ok(RTCSessionDescription::offer(sdp.to_string())
            .map_err(|e| anyhow!("invalid offer SDP: {e}"))?)
    } else {
        Ok(RTCSessionDescription::answer(sdp.to_string())
            .map_err(|e| anyhow!("invalid answer SDP: {e}"))?)
    }
}

fn to_rtc_ice(cfg: &IceServerConfig) -> RTCIceServer {
    RTCIceServer {
        urls: cfg.urls.clone(),
        username: cfg.username.clone().unwrap_or_default(),
        credential: cfg.credential.clone().unwrap_or_default(),
        ..Default::default()
    }
}
