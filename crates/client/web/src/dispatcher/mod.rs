use std::collections::{HashMap, HashSet};

use crate::utils::to_js_object;
use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;

pub type CallbackId = u64;

type ConnectedCallback = fn();
type MessageCallback<Msg> = fn(PeerID, Msg);
type DisconnectedCallback = fn();
type PeerConnectedCallback = fn(PeerID);
type PeerDisconnectedCallback = fn(PeerID);
type PeerAvailableCallback = fn();

#[derive(Clone)]
pub enum RtcEvent<Msg: UserMsgPayload> {
    Connected,
    UserMessage(PeerID, Msg),
    Disconnected,
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
    PeerAvailable,
}

pub enum Rtc<Msg: UserMsgPayload> {
    Connected(ConnectedCallback),
    UserMessage(MessageCallback<Msg>),
    Disconnected(DisconnectedCallback),
    PeerConnected(PeerConnectedCallback),
    PeerDisconnected(PeerDisconnectedCallback),
    PeerAvailable(PeerAvailableCallback),
    JsConnected(js_sys::Function),
    JsUserMessage(js_sys::Function),
    JsDisconnected(js_sys::Function),
    JsPeerConnected(js_sys::Function),
    JsPeerDisconnected(js_sys::Function),
    JsPeerAvailable(js_sys::Function),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SubscriptionKind {
    Connected,
    UserMessage,
    Disconnected,
    PeerConnected,
    PeerDisconnected,
    PeerAvailable,
}

impl<Msg: UserMsgPayload> Rtc<Msg> {
    fn kind(&self) -> SubscriptionKind {
        match self {
            Self::Connected(_) | Self::JsConnected(_) => SubscriptionKind::Connected,
            Self::UserMessage(_) | Self::JsUserMessage(_) => SubscriptionKind::UserMessage,
            Self::Disconnected(_) | Self::JsDisconnected(_) => SubscriptionKind::Disconnected,
            Self::PeerConnected(_) | Self::JsPeerConnected(_) => SubscriptionKind::PeerConnected,
            Self::PeerDisconnected(_) | Self::JsPeerDisconnected(_) => {
                SubscriptionKind::PeerDisconnected
            }
            Self::PeerAvailable(_) | Self::JsPeerAvailable(_) => SubscriptionKind::PeerAvailable,
        }
    }
}

impl<Msg: UserMsgPayload> RtcEvent<Msg> {
    fn kind(&self) -> SubscriptionKind {
        match self {
            Self::Connected => SubscriptionKind::Connected,
            Self::UserMessage(_, _) => SubscriptionKind::UserMessage,
            Self::Disconnected => SubscriptionKind::Disconnected,
            Self::PeerConnected(_) => SubscriptionKind::PeerConnected,
            Self::PeerDisconnected(_) => SubscriptionKind::PeerDisconnected,
            Self::PeerAvailable => SubscriptionKind::PeerAvailable,
        }
    }
}

pub trait Dispatcher<Msg: UserMsgPayload> {
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()>;
}

pub struct RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    next_callback_id: CallbackId,
    subscriptions: HashMap<CallbackId, Rtc<Msg>>,
    subscriptions_by_kind: HashMap<SubscriptionKind, HashSet<CallbackId>>,
}

impl<Msg> RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    pub fn new() -> Self {
        Self {
            next_callback_id: 1,
            subscriptions: HashMap::new(),
            subscriptions_by_kind: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> CallbackId {
        let id = self.next_callback_id;
        self.next_callback_id += 1;
        id
    }

    pub fn subscribe(&mut self, subscription: Rtc<Msg>) -> CallbackId {
        let id = self.next_id();
        let kind = subscription.kind();

        self.subscriptions.insert(id, subscription);
        self.subscriptions_by_kind
            .entry(kind)
            .or_default()
            .insert(id);

        id
    }

    pub fn unsubscribe(&mut self, id: CallbackId) -> bool {
        let Some(subscription) = self.subscriptions.remove(&id) else {
            return false;
        };

        let kind = subscription.kind();
        if let Some(ids) = self.subscriptions_by_kind.get_mut(&kind) {
            ids.remove(&id);
            if ids.is_empty() {
                self.subscriptions_by_kind.remove(&kind);
            }
        }

        true
    }
}

impl<Msg> Dispatcher<Msg> for RtcCallbacks<Msg>
where
    Msg: UserMsgPayload,
{
    fn emit(&self, event: RtcEvent<Msg>) -> Result<()> {
        let Some(ids) = self.subscriptions_by_kind.get(&event.kind()) else {
            return Ok(());
        };

        let ids: Vec<CallbackId> = ids.iter().copied().collect();

        for id in ids {
            let Some(subscription) = self.subscriptions.get(&id) else {
                continue;
            };

            match (subscription, &event) {
                (Rtc::Connected(cb), RtcEvent::Connected) => cb(),
                (Rtc::UserMessage(cb), RtcEvent::UserMessage(peer, data)) => {
                    cb(peer.clone(), data.clone())
                }
                (Rtc::Disconnected(cb), RtcEvent::Disconnected) => cb(),
                (Rtc::PeerConnected(cb), RtcEvent::PeerConnected(peer)) => cb(peer.clone()),
                (Rtc::PeerDisconnected(cb), RtcEvent::PeerDisconnected(peer)) => cb(peer.clone()),
                (Rtc::JsConnected(cb), RtcEvent::Connected) => {
                    cb.call0(&JsValue::NULL)
                        .map_err(|e| anyhow!("Failed to call onConnected callback: {:#?}", e))?;
                }
                (Rtc::JsUserMessage(cb), RtcEvent::UserMessage(peer, data)) => {
                    let msg_obj = to_js_object(data)
                        .map_err(|e| anyhow!("Failed to serialize message payload {:#?}", e))?;
                    let peer = js_sys::JsString::from(peer.as_str());
                    cb.call2(&JsValue::NULL, &peer, &msg_obj)
                        .map_err(|e| anyhow!("Failed to call onMessage callback: {:#?}", e))?;
                }
                (Rtc::JsDisconnected(cb), RtcEvent::Disconnected) => {
                    cb.call0(&JsValue::NULL).ok();
                }
                (Rtc::JsPeerConnected(cb), RtcEvent::PeerConnected(peer)) => {
                    let peer = js_sys::JsString::from(peer.as_str());
                    cb.call1(&JsValue::NULL, &peer).ok();
                }
                (Rtc::JsPeerDisconnected(cb), RtcEvent::PeerDisconnected(peer)) => {
                    let peer = js_sys::JsString::from(peer.as_str());
                    cb.call1(&JsValue::NULL, &peer).ok();
                }
                (Rtc::PeerAvailable(cb), RtcEvent::PeerAvailable) => cb(),
                (Rtc::JsPeerAvailable(cb), RtcEvent::PeerAvailable) => {
                    cb.call0(&JsValue::NULL).ok();
                }
                _ => {}
            }
        }

        Ok(())
    }
}
