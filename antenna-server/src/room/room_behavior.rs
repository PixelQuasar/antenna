use crate::room::context::RoomContext;
use antenna_core::PeerId;
use async_trait::async_trait;
use bytes::Bytes;

#[async_trait]
pub trait RoomBehavior: Send + Sync + 'static {
    async fn on_join(&self, ctx: &RoomContext, peer_id: PeerId);

    async fn on_message(&self, ctx: &RoomContext, peer_id: PeerId, data: Bytes);

    async fn on_leave(&self, ctx: &RoomContext, peer_id: PeerId);
}
