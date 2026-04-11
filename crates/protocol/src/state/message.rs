use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{HandshakeInput, PeerID};

pub trait UserMsgPayload: Serialize + DeserializeOwned + Clone + 'static {}

impl<T> UserMsgPayload for T where T: Serialize + DeserializeOwned + Clone + 'static {}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub enum MsgPayload<Msg: UserMsgPayload> {
    User(Msg),
    PeerJoined(PeerID),
    RelaySignaling {
        via: PeerID,
        dst: PeerID,
        data: HandshakeInput,
    },
    PeerLeft(PeerID),
    Heartbeat,
}
