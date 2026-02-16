use antenna_core::PeerId;
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::error;
use webrtc::data_channel::RTCDataChannel;

#[derive(Clone)]
pub struct RoomContext {
    peers: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>,
}

impl RoomContext {
    pub(crate) fn new(peers: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>) -> Self {
        Self { peers }
    }

    pub async fn send(&self, peer_id: &PeerId, data: Bytes) {
        if let Some(peer) = self.peers.get(peer_id) {
            println!("sending data {:?}", data);

            if let Err(e) = peer.send(&data).await {
                error!("Failed to send message to user {:?}: {}", peer_id, e);
            }
        } else {
            error!(
                "Attempted to send message to disconnected user {:?}",
                peer_id
            );
        }
    }

    pub async fn broadcast(&self, data: Bytes) {
        let mut channels = Vec::new();
        for entry in self.peers.iter() {
            channels.push(entry.value().clone());
        }

        for channel in channels {
            let data_clone = data.clone();
            tokio::spawn(async move {
                if let Err(e) = channel.send(&data_clone).await {
                    error!("Broadcast error: {}", e);
                }
            });
        }
    }

    pub fn list_users(&self) -> Vec<PeerId> {
        self.peers.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn contains_user(&self, peer_id: &PeerId) -> bool {
        self.peers.contains_key(peer_id)
    }
}
