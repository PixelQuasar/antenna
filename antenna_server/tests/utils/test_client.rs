use anyhow::{Context, Result};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use antenna_core::PeerId;

/// Configuration for TestClient.
#[derive(Clone)]
pub struct TestClientConfig {
    /// ICE servers to use (default: none for local testing).
    pub ice_servers: Vec<String>,
}

impl Default for TestClientConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![],
        }
    }
}

pub struct TestClient {
    /// The peer ID for this client.
    pub peer_id: PeerId,
    /// The underlying RTCPeerConnection.
    peer_connection: Arc<RTCPeerConnection>,
    /// The data channel (created on offer side).
    data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    /// Received messages.
    received_messages: Arc<Mutex<Vec<Bytes>>>,
    /// Channel to notify when data channel is open.
    dc_open_tx: mpsc::Sender<()>,
    dc_open_rx: Arc<Mutex<mpsc::Receiver<()>>>,
    /// Channel to notify when connection state changes.
    connection_state: Arc<Mutex<RTCPeerConnectionState>>,
    /// Generated ICE candidates (to be sent to the server).
    ice_candidates: Arc<Mutex<Vec<String>>>,
}

impl TestClient {
    /// Create a new TestClient with the given peer ID and configuration.
    pub async fn new(peer_id: PeerId, config: TestClientConfig) -> Result<Self> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;

        let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        let ice_servers = if config.ice_servers.is_empty() {
            vec![]
        } else {
            vec![RTCIceServer {
                urls: config.ice_servers,
                ..Default::default()
            }]
        };

        let rtc_config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

        let (dc_open_tx, dc_open_rx) = mpsc::channel(1);
        let (ice_tx, _ice_rx) = mpsc::unbounded_channel();
        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>> = Arc::new(Mutex::new(None));
        let connection_state = Arc::new(Mutex::new(RTCPeerConnectionState::New));
        let ice_candidates = Arc::new(Mutex::new(Vec::new()));

        let state_clone = Arc::clone(&connection_state);
        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let state_clone = Arc::clone(&state_clone);
            Box::pin(async move {
                tracing::debug!("[TestClient] Connection state: {:?}", state);
                *state_clone.lock().await = state;
            })
        }));

        let ice_tx_clone = ice_tx.clone();
        let ice_candidates_clone = Arc::clone(&ice_candidates);
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let ice_tx = ice_tx_clone.clone();
            let ice_candidates = Arc::clone(&ice_candidates_clone);
            Box::pin(async move {
                if let Some(c) = candidate {
                    if let Ok(json) = c.to_json() {
                        if let Ok(s) = serde_json::to_string(&json) {
                            tracing::debug!("[TestClient] ICE candidate generated");
                            ice_candidates.lock().await.push(s.clone());
                            let _ = ice_tx.send(s);
                        }
                    }
                }
            })
        }));

        let dc_clone = Arc::clone(&data_channel);
        let messages_clone = Arc::clone(&received_messages);
        let dc_open_tx_clone = dc_open_tx.clone();
        peer_connection.on_data_channel(Box::new(move |dc| {
            let dc_clone = Arc::clone(&dc_clone);
            let messages_clone = Arc::clone(&messages_clone);
            let dc_open_tx = dc_open_tx_clone.clone();

            Box::pin(async move {
                tracing::debug!("[TestClient] Data channel received: {}", dc.label());

                *dc_clone.lock().await = Some(Arc::clone(&dc));

                let dc_open_tx2 = dc_open_tx.clone();
                dc.on_open(Box::new(move || {
                    let dc_open_tx = dc_open_tx2.clone();
                    Box::pin(async move {
                        tracing::debug!("[TestClient] Data channel opened");
                        let _ = dc_open_tx.send(()).await;
                    })
                }));

                let messages = Arc::clone(&messages_clone);
                dc.on_message(Box::new(move |msg: DataChannelMessage| {
                    let messages = Arc::clone(&messages);
                    Box::pin(async move {
                        let data = Bytes::from(msg.data.to_vec());
                        tracing::debug!("[TestClient] Message received: {} bytes", data.len());
                        messages.lock().await.push(data);
                    })
                }));
            })
        }));

        Ok(Self {
            peer_id,
            peer_connection,
            data_channel,
            received_messages,
            dc_open_tx,
            dc_open_rx: Arc::new(Mutex::new(dc_open_rx)),
            connection_state,
            ice_candidates,
        })
    }

    /// Create an SDP offer and a data channel.
    ///
    /// Returns the SDP offer string to be sent to the server.
    pub async fn create_offer(&self) -> Result<String> {
        let dc = self
            .peer_connection
            .create_data_channel("data", None)
            .await
            .context("Failed to create data channel")?;

        let messages = Arc::clone(&self.received_messages);
        dc.on_message(Box::new(move |msg: DataChannelMessage| {
            let messages = Arc::clone(&messages);
            Box::pin(async move {
                let data = Bytes::from(msg.data.to_vec());
                tracing::debug!("[TestClient] Message received: {} bytes", data.len());
                messages.lock().await.push(data);
            })
        }));

        let dc_open_tx = self.dc_open_tx.clone();
        dc.on_open(Box::new(move || {
            let dc_open_tx = dc_open_tx.clone();
            Box::pin(async move {
                tracing::debug!("[TestClient] Data channel opened (offerer)");
                let _ = dc_open_tx.send(()).await;
            })
        }));

        *self.data_channel.lock().await = Some(dc);

        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .context("Failed to create offer")?;

        self.peer_connection
            .set_local_description(offer.clone())
            .await
            .context("Failed to set local description")?;

        Ok(offer.sdp)
    }

    /// Wait for ICE gathering to complete and return all candidates.
    pub async fn gather_ice_candidates(&self, timeout_ms: u64) -> Result<Vec<String>> {
        let mut gathering_complete = self.peer_connection.gathering_complete_promise().await;

        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            gathering_complete.recv(),
        )
        .await;

        match timeout_result {
            Ok(_) => {
                let candidates = self.ice_candidates.lock().await.clone();
                tracing::debug!(
                    "[TestClient] ICE gathering complete: {} candidates",
                    candidates.len()
                );
                Ok(candidates)
            }
            Err(_) => {
                let candidates = self.ice_candidates.lock().await.clone();
                tracing::warn!(
                    "[TestClient] ICE gathering timeout, returning {} candidates",
                    candidates.len()
                );
                Ok(candidates)
            }
        }
    }

    /// Set the remote SDP answer received from the server.
    pub async fn set_remote_answer(&self, sdp: String) -> Result<()> {
        let answer =
            webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(sdp)?;
        self.peer_connection
            .set_remote_description(answer)
            .await
            .context("Failed to set remote description")?;
        Ok(())
    }

    /// Add a remote ICE candidate received from the server.
    pub async fn add_ice_candidate(&self, candidate_json: String) -> Result<()> {
        let candidate: webrtc::ice_transport::ice_candidate::RTCIceCandidateInit =
            serde_json::from_str(&candidate_json).context("Failed to parse ICE candidate")?;
        self.peer_connection
            .add_ice_candidate(candidate)
            .await
            .context("Failed to add ICE candidate")?;
        Ok(())
    }

    /// Wait for the data channel to be open.
    pub async fn wait_for_data_channel(&self, timeout_ms: u64) -> Result<()> {
        let mut rx = self.dc_open_rx.lock().await;
        let timeout_result =
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx.recv()).await;

        match timeout_result {
            Ok(Some(())) => Ok(()),
            Ok(None) => anyhow::bail!("Data channel open channel closed"),
            Err(_) => anyhow::bail!("Timeout waiting for data channel to open"),
        }
    }

    /// Wait for the connection to be established.
    pub async fn wait_for_connection(&self, timeout_ms: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            let state = *self.connection_state.lock().await;
            match state {
                RTCPeerConnectionState::Connected => return Ok(()),
                RTCPeerConnectionState::Failed => {
                    anyhow::bail!("Connection failed")
                }
                RTCPeerConnectionState::Closed => {
                    anyhow::bail!("Connection closed")
                }
                _ => {}
            }

            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for connection (state: {:?})", state);
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    /// Send a binary message through the data channel.
    pub async fn send_message(&self, data: &[u8]) -> Result<()> {
        let dc = self
            .data_channel
            .lock()
            .await
            .clone()
            .context("Data channel not available")?;

        dc.send(&Bytes::from(data.to_vec()))
            .await
            .context("Failed to send message")?;

        Ok(())
    }

    /// Close the peer connection.
    pub async fn close(&self) -> Result<()> {
        self.peer_connection
            .close()
            .await
            .context("Failed to close peer connection")?;
        Ok(())
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // Note: We can't call async close() in drop, but the peer connection
        // will be cleaned up when all Arc references are dropped.
        tracing::debug!("[TestClient] Dropping client {:?}", self.peer_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let peer_id = PeerId::new();
        let client = TestClient::new(peer_id.clone(), TestClientConfig::default())
            .await
            .expect("Failed to create test client");

        assert_eq!(client.peer_id, peer_id);
    }

    #[tokio::test]
    async fn test_client_creates_offer() {
        let peer_id = PeerId::new();
        let client = TestClient::new(peer_id, TestClientConfig::default())
            .await
            .expect("Failed to create test client");

        let offer = client.create_offer().await.expect("Failed to create offer");

        assert!(!offer.is_empty());
        assert!(offer.contains("v=0")); // SDP starts with version
    }
}
