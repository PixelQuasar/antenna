use std::collections::HashMap;

use antenna_protocol::{HandshakeInput, PeerID, UserMsgPayload};
use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;

use crate::utils::to_js_object;

pub type CallbackId = u64;

type ConnectedCallback = fn();
type MessageCallback<Msg> = fn(PeerID, Msg);
type DisconnectedCallback = fn();
type PeerConnectedCallback = fn(PeerID);
type PeerDisconnectedCallback = fn(PeerID);
type SignalingMessageCallback = fn(PeerID, HandshakeInput);

#[derive(Clone)]
pub enum RtcEvent<Msg: UserMsgPayload> {
    Connected,
    UserMessage(PeerID, Msg),
    SignalingMessage(PeerID, HandshakeInput),
    Disconnected,
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
}

pub enum Rtc<Msg: UserMsgPayload> {
    Connected(ConnectedCallback),
    UserMessage(MessageCallback<Msg>),
    SignalingMessage(SignalingMessageCallback),
    Disconnected(DisconnectedCallback),
    PeerConnected(PeerConnectedCallback),
    PeerDisconnected(PeerDisconnectedCallback),
    JsConnected(js_sys::Function),
    JsUserMessage(js_sys::Function),
    JsDisconnected(js_sys::Function),
    JsPeerConnected(js_sys::Function),
    JsPeerDisconnected(js_sys::Function),
}

#[derive(Clone, Copy)]
enum SubscriptionKind {
    Connected,
    UserMessage,
    SignalingMessage,
    Disconnected,
    PeerConnected,
    PeerDisconnected,
}

pub trait Dispatcher<Msg: UserMsgPayload> {
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()>;
}

pub struct RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    next_callback_id: CallbackId,
    id_map: HashMap<CallbackId, SubscriptionKind>,
    connected_map: HashMap<CallbackId, ConnectedCallback>,
    message_map: HashMap<CallbackId, MessageCallback<Msg>>,
    signaling_message_map: HashMap<CallbackId, SignalingMessageCallback>,
    disconnected_map: HashMap<CallbackId, DisconnectedCallback>,
    peer_connected_map: HashMap<CallbackId, PeerConnectedCallback>,
    peer_disconnected_map: HashMap<CallbackId, PeerDisconnectedCallback>,
    js_connected_map: HashMap<CallbackId, js_sys::Function>,
    js_message_map: HashMap<CallbackId, js_sys::Function>,
    js_disconnected_map: HashMap<CallbackId, js_sys::Function>,
    js_peer_connected_map: HashMap<CallbackId, js_sys::Function>,
    js_peer_disconnected_map: HashMap<CallbackId, js_sys::Function>,
}

impl<Msg> RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    pub fn new() -> Self {
        Self {
            next_callback_id: 1,
            id_map: HashMap::new(),
            connected_map: HashMap::new(),
            message_map: HashMap::new(),
            signaling_message_map: HashMap::new(),
            disconnected_map: HashMap::new(),
            peer_connected_map: HashMap::new(),
            peer_disconnected_map: HashMap::new(),
            js_connected_map: HashMap::new(),
            js_message_map: HashMap::new(),
            js_disconnected_map: HashMap::new(),
            js_peer_connected_map: HashMap::new(),
            js_peer_disconnected_map: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> CallbackId {
        let id = self.next_callback_id;
        self.next_callback_id += 1;
        id
    }

    pub fn subscribe(&mut self, subscription: Rtc<Msg>) -> CallbackId {
        let id = self.next_id();

        match subscription {
            Rtc::Connected(cb) => {
                self.id_map.insert(id, SubscriptionKind::Connected);
                self.connected_map.insert(id, cb);
            }
            Rtc::UserMessage(cb) => {
                self.id_map.insert(id, SubscriptionKind::UserMessage);
                self.message_map.insert(id, cb);
            }
            Rtc::SignalingMessage(cb) => {
                self.id_map.insert(id, SubscriptionKind::SignalingMessage);
                self.signaling_message_map.insert(id, cb);
            }
            Rtc::Disconnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::Disconnected);
                self.disconnected_map.insert(id, cb);
            }
            Rtc::PeerConnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::PeerConnected);
                self.peer_connected_map.insert(id, cb);
            }
            Rtc::PeerDisconnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::PeerDisconnected);
                self.peer_disconnected_map.insert(id, cb);
            }
            Rtc::JsConnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::Connected);
                self.js_connected_map.insert(id, cb);
            }
            Rtc::JsUserMessage(cb) => {
                self.id_map.insert(id, SubscriptionKind::UserMessage);
                self.js_message_map.insert(id, cb);
            }
            Rtc::JsDisconnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::Disconnected);
                self.js_disconnected_map.insert(id, cb);
            }
            Rtc::JsPeerConnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::PeerConnected);
                self.js_peer_connected_map.insert(id, cb);
            }
            Rtc::JsPeerDisconnected(cb) => {
                self.id_map.insert(id, SubscriptionKind::PeerDisconnected);
                self.js_peer_disconnected_map.insert(id, cb);
            }
        }
        id
    }

    pub fn unsubscribe(&mut self, id: CallbackId) -> bool {
        let Some(kind) = self.id_map.remove(&id) else {
            return false;
        };

        match kind {
            SubscriptionKind::Connected => {
                self.connected_map.remove(&id).is_some()
                    || self.js_connected_map.remove(&id).is_some()
            }
            SubscriptionKind::UserMessage => {
                self.message_map.remove(&id).is_some() || self.js_message_map.remove(&id).is_some()
            }
            SubscriptionKind::SignalingMessage => self.signaling_message_map.remove(&id).is_some(),
            SubscriptionKind::Disconnected => {
                self.disconnected_map.remove(&id).is_some()
                    || self.js_disconnected_map.remove(&id).is_some()
            }
            SubscriptionKind::PeerConnected => {
                self.peer_connected_map.remove(&id).is_some()
                    || self.js_peer_connected_map.remove(&id).is_some()
            }
            SubscriptionKind::PeerDisconnected => {
                self.peer_disconnected_map.remove(&id).is_some()
                    || self.js_peer_disconnected_map.remove(&id).is_some()
            }
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
                for callback in self.connected_map.values() {
                    callback();
                }
                for callback in self.js_connected_map.values() {
                    callback
                        .call0(&JsValue::NULL)
                        .map_err(|e| anyhow!("Failed to call onConnected callback: {:#?}", e))?;
                }
            }
            RtcEvent::UserMessage(peer, data) => {
                for callback in self.message_map.values() {
                    callback(peer.clone(), data.clone());
                }
                let msg_obj = to_js_object(&data)
                    .map_err(|e| anyhow!("Failed to serialize message payload {:#?}", e))?;
                let peer = js_sys::JsString::from(peer.as_str());
                for callback in self.js_message_map.values() {
                    callback
                        .call2(&JsValue::NULL, &peer, &msg_obj)
                        .map_err(|e| anyhow!("Failed to call onMessage callback: {:#?}", e))?;
                }
            }
            RtcEvent::SignalingMessage(peer, data) => {
                for callback in self.signaling_message_map.values() {
                    callback(peer.clone(), data.clone());
                }
            }
            RtcEvent::Disconnected => {
                for callback in self.disconnected_map.values() {
                    callback();
                }
                for callback in self.js_disconnected_map.values() {
                    callback.call0(&JsValue::NULL).ok();
                }
            }
            RtcEvent::PeerConnected(peer) => {
                for callback in self.peer_connected_map.values() {
                    callback(peer.clone());
                }
                let peer = js_sys::JsString::from(peer.as_str());
                for callback in self.js_peer_connected_map.values() {
                    callback.call1(&JsValue::NULL, &peer).ok();
                }
            }
            RtcEvent::PeerDisconnected(peer) => {
                for callback in self.peer_disconnected_map.values() {
                    callback(peer.clone());
                }
                let peer = js_sys::JsString::from(peer.as_str());
                for callback in self.js_peer_disconnected_map.values() {
                    callback.call1(&JsValue::NULL, &peer).ok();
                }
            }
        }
        Ok(())
    }
}
