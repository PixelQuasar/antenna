mod async_callback;
mod callbacks;
mod config;

pub use async_callback::async_callback;
pub use callbacks::{
    Dispatcher, MessageCallback, PeerConnectedCallback, PeerDisconnectedCallback, RtcCallbacks,
    RtcEvent,
};
pub use config::IceServerConfig;

pub type Msg = Vec<u8>; // TODO REMOVE LATER! hardcode
