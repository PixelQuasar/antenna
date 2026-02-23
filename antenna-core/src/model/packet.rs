use crate::model::peer::PeerId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum Packet<T> {
    System(SystemMessage),
    User(T),
    RpcResponse {
        req_id: Uuid,
        #[serde(with = "serde_bytes")]
        payload: Vec<u8>,
        is_error: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemMessage {
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
    PeerLeft(PeerId),
    PeerJoined(PeerId),
}
