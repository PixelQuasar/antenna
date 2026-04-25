use std::collections::HashSet;

use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::Result;
use async_trait::async_trait;

use crate::CallbackId;

#[async_trait(?Send)]
pub trait Peer<Msg: UserMsgPayload> {
    type Subscription;

    fn my_id(&self) -> PeerID;
    fn subscribe(&mut self, subscription: Self::Subscription) -> CallbackId;
    fn unsubscribe(&mut self, id: CallbackId) -> bool;
    async fn start(&self) -> Result<String>;
    async fn receive_offer(&self, offer: &str) -> Result<String>;
    async fn receive_answer(&self, answer: &str) -> Result<()>;
    fn send(&self, peer_id: PeerID, data: Msg);
    fn broadcast(&self, data: Msg);
    fn is_connected(&self, peer_id: PeerID) -> bool;
    fn connected_peers(&self) -> HashSet<String>;
}
