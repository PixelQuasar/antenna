use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{PeerID, SignalingPayload};

pub trait UserMsgPayload: Serialize + DeserializeOwned + Clone + 'static {}

impl<T> UserMsgPayload for T where T: Serialize + DeserializeOwned + Clone + 'static {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RelayPayload {
    InitHost,
    InitJoiner,
    Signaling(SignalingPayload),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub enum MsgPayload<Msg: UserMsgPayload> {
    User(Msg),
    PeerJoined(PeerID),
    RelaySignalingTo { dst: PeerID, data: RelayPayload },
    RelaySignalingFrom { src: PeerID, data: RelayPayload },
    PeerLeft(PeerID),
    Heartbeat,
}
