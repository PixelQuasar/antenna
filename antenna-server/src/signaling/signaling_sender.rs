use antenna_core::PeerId;
use async_trait::async_trait;

#[async_trait]
pub trait SignalingSender: Send + Sync {
    async fn send_answer(&self, peer_id: PeerId, sdp: String);

    async fn send_ice(&self, peer_id: PeerId, candidate: String);
}
