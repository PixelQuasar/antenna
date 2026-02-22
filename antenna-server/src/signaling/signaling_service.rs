use crate::room::RoomCommand;
use crate::signaling::SignalingOutput;
use antenna_core::{IceServerConfig, PeerId, SignalMessage};
use async_trait::async_trait;
use axum::extract::ws::Message;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};

struct SignalingInner {
    peers: DashMap<PeerId, mpsc::UnboundedSender<Message>>,
    ice_servers: Vec<IceServerConfig>,
}

#[derive(Clone)]
pub struct SignalingService {
    inner: Arc<SignalingInner>,
    pub(crate) room_cmd_tx: mpsc::Sender<RoomCommand>,
}

impl SignalingService {
    pub fn new(room_cmd_tx: mpsc::Sender<RoomCommand>, ice_servers: Vec<IceServerConfig>) -> Self {
        Self {
            inner: Arc::new(SignalingInner {
                peers: DashMap::new(),
                ice_servers,
            }),
            room_cmd_tx,
        }
    }

    pub fn get_ice_servers(&self) -> Vec<IceServerConfig> {
        self.inner.ice_servers.clone()
    }

    pub fn add_peer(&self, peer_id: PeerId, tx: mpsc::UnboundedSender<Message>) {
        self.inner.peers.insert(peer_id, tx);
    }

    pub fn remove_peer(&self, peer_id: &PeerId) {
        self.inner.peers.remove(peer_id);
    }

    pub fn send_signal(&self, peer_id: PeerId, msg: SignalMessage) {
        if let Some(peer) = self.inner.peers.get(&peer_id) {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(e) = peer.send(Message::Text(json.into())) {
                        error!("Failed to send WS message to {:?}: {:?}", peer_id, e);
                    }
                }
                Err(e) => error!("Failed to serialize signal message: {}", e),
            }
        } else {
            warn!(
                "Attempted to send signal to disconnected user {:?}",
                peer_id
            );
        }
    }
}

#[async_trait]
impl SignalingOutput for SignalingService {
    async fn send_answer(&self, peer_id: PeerId, sdp: String) {
        let msg = SignalMessage::Answer { sdp };
        self.send_signal(peer_id, msg);
    }

    async fn send_ice(&self, peer_id: PeerId, candidate: String) {
        let msg = SignalMessage::IceCandidate {
            candidate,
            sdp_mid: None,
            sdp_m_line_index: None,
        };
        self.send_signal(peer_id, msg);
    }
}
