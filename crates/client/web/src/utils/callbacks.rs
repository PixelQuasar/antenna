use antenna_protocol::{HandshakeInput, PeerID, UserMsgPayload};
use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;

use crate::utils::to_js_object;

pub enum RtcEvent<Msg: UserMsgPayload> {
    Connected,

    UserMessage(PeerID, Msg),

    SignalingMessage(PeerID, HandshakeInput),

    Disconnected,

    PeerConnected(PeerID),

    PeerDisconnected(PeerID),
}

pub trait Dispatcher<Msg: UserMsgPayload> {
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()>;
}

pub type ConnectedCallback = fn();

pub type MessageCallback<Msg> = fn(PeerID, Msg);

pub type DisconnectedCallback = fn();

pub type PeerConnectedCallback = fn(PeerID);

pub type PeerDisconnectedCallback = fn(PeerID);

pub type SignalingMessageCallback = fn(PeerID, HandshakeInput);

#[derive(Clone)]
pub struct RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    pub on_connected: Option<ConnectedCallback>,

    pub on_message: Option<MessageCallback<Msg>>,

    pub on_disconnected: Option<DisconnectedCallback>,

    pub on_peer_connected: Option<PeerConnectedCallback>,

    pub on_peer_disconnected: Option<PeerDisconnectedCallback>,

    pub on_signaling_message: Option<SignalingMessageCallback>,

    pub js_on_connected: Option<js_sys::Function>,

    pub js_on_message: Option<js_sys::Function>,

    pub js_on_disconnected: Option<js_sys::Function>,

    pub js_on_peer_connected: Option<js_sys::Function>,

    pub js_on_peer_disconnected: Option<js_sys::Function>,
}

impl<Msg> RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    pub fn new() -> Self {
        RtcCallbacks {
            on_connected: None,
            js_on_connected: None,
            on_message: None,
            js_on_message: None,
            on_disconnected: None,
            js_on_disconnected: None,
            on_peer_connected: None,
            js_on_peer_connected: None,
            on_peer_disconnected: None,
            js_on_peer_disconnected: None,
            on_signaling_message: None,
        }
    }
}

impl<Msg> Dispatcher<Msg> for RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()> {
        match event {
            RtcEvent::Connected => {
                if let Some(on_connected) = self.on_connected {
                    on_connected();
                }
                if let Some(on_connected) = &self.js_on_connected {
                    on_connected
                        .call0(&JsValue::NULL)
                        .map_err(|e| anyhow!("Failed to call onConnected callback: {:#?}", e))?;
                }
            }
            RtcEvent::UserMessage(peer, data) => {
                if let Some(on_message) = self.on_message {
                    on_message(peer.clone(), data.clone());
                }
                if let Some(on_message) = &self.js_on_message {
                    let msg_obj = to_js_object(&data)
                        .map_err(|e| anyhow!("Failed to serialize message payload {:#?}", e))?;
                    let peer = js_sys::JsString::from(peer.as_str());
                    on_message
                        .call2(&JsValue::NULL, &peer, &msg_obj)
                        .map_err(|e| anyhow!("Failed to call onMessage callback: {:#?}", e))?;
                }
            }
            RtcEvent::SignalingMessage(peer, data) => {
                if let Some(on_signaling_message) = self.on_signaling_message {
                    on_signaling_message(peer.clone(), data.clone());
                }
            }
            RtcEvent::Disconnected => {
                if let Some(on_disconnected) = self.on_disconnected {
                    on_disconnected();
                }
                if let Some(on_disconnected) = &self.js_on_disconnected {
                    on_disconnected.call0(&JsValue::NULL).ok();
                }
            }
            RtcEvent::PeerConnected(peer) => {
                if let Some(on_peer_connected) = self.on_peer_connected {
                    on_peer_connected(peer.clone());
                }
                if let Some(on_peer_connected) = &self.js_on_peer_connected {
                    let peer = js_sys::JsString::from(peer.as_str());
                    on_peer_connected.call1(&JsValue::NULL, &peer).ok();
                }
            }
            RtcEvent::PeerDisconnected(peer) => {
                if let Some(on_peer_disconnected) = self.on_peer_disconnected {
                    on_peer_disconnected(peer.clone());
                }
                if let Some(on_peer_disconnected) = &self.js_on_peer_disconnected {
                    let peer = js_sys::JsString::from(peer.as_str());
                    on_peer_disconnected.call1(&JsValue::NULL, &peer).ok();
                }
            }
        }
        Ok(())
    }
}
