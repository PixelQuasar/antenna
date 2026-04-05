use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::PeerID;

pub trait UserMsgPayload: Serialize + DeserializeOwned + Clone + 'static {}

impl<T> UserMsgPayload for T where T: Serialize + DeserializeOwned + Clone + 'static {}

#[derive(Clone, Serialize, Deserialize)]
pub enum SignalingRole {
    Offer,
    Answer,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub enum Payload<Msg: UserMsgPayload> {
    User(Msg),
    Signaling { role: SignalingRole, sdp: String },
    Heartbeat,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub struct Message<Msg: UserMsgPayload> {
    peer_to: PeerID,
    payload: Payload<Msg>,
}
