use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::Result;

pub type CallbackId = u64;

#[derive(Clone)]
pub enum RtcEvent<Msg: UserMsgPayload> {
    Connected,
    UserMessage(PeerID, Msg),
    Disconnected,
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
    Available,
    Unavailable,
}

pub trait Dispatcher<Msg: UserMsgPayload> {
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()>;
}
