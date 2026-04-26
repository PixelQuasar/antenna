use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{PeerID, SignalingPayload};

pub trait UserMsgPayload: Serialize + DeserializeOwned + Clone {}

impl<T> UserMsgPayload for T where T: Serialize + DeserializeOwned + Clone {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RelayPayload {
    InitHost(PeerID),
    InitJoiner(PeerID),
    Offer(SignalingPayload),
    Answer(SignalingPayload),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub enum MsgPayload<Msg: UserMsgPayload> {
    User(Msg),
    RelaySignalingTo { dst: PeerID, data: RelayPayload },
    RelaySignalingFrom { src: PeerID, data: RelayPayload },
    Heartbeat,
    Disconnect,
}
