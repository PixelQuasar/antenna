use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub enum EventType<Msg: UserMsgPayload> {
    Connected,
    UserMessage(PeerID, Msg),
    Disconnected,
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
    PeerDropped(PeerID),
    Available,
    Unavailable,
}

pub struct NoArgCallback(Box<dyn Fn() -> Result<()>>);

impl NoArgCallback {
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn() -> Result<()> + 'static,
    {
        Self(Box::new(f))
    }

    pub fn call(&self) -> Result<()> {
        (self.0)()
    }
}

impl From<fn()> for NoArgCallback {
    fn from(f: fn()) -> Self {
        Self(Box::new(move || {
            f();
            Ok(())
        }))
    }
}

pub struct PeerCallback(Box<dyn Fn(&PeerID) -> Result<()>>);

impl PeerCallback {
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&PeerID) -> Result<()> + 'static,
    {
        Self(Box::new(f))
    }

    pub fn call(&self, peer: &PeerID) -> Result<()> {
        (self.0)(peer)
    }
}

impl From<fn(PeerID)> for PeerCallback {
    fn from(f: fn(PeerID)) -> Self {
        Self(Box::new(move |peer| {
            f(peer.clone());
            Ok(())
        }))
    }
}

pub struct MessageCallback<Msg: UserMsgPayload>(Box<dyn Fn(&PeerID, &Msg) -> Result<()>>);

impl<Msg: UserMsgPayload> MessageCallback<Msg> {
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(&PeerID, &Msg) -> Result<()> + 'static,
    {
        Self(Box::new(f))
    }

    pub fn call(&self, peer: &PeerID, data: &Msg) -> Result<()> {
        (self.0)(peer, data)
    }
}

impl<Msg: UserMsgPayload + 'static> From<fn(PeerID, Msg)> for MessageCallback<Msg> {
    fn from(f: fn(PeerID, Msg)) -> Self {
        Self(Box::new(move |peer, data| {
            f(peer.clone(), data.clone());
            Ok(())
        }))
    }
}

pub enum Event<Msg: UserMsgPayload> {
    Connected(NoArgCallback),
    UserMessage(MessageCallback<Msg>),
    Disconnected(NoArgCallback),
    PeerConnected(PeerCallback),
    PeerDisconnected(PeerCallback),
    PeerLost(PeerCallback),
    Available(NoArgCallback),
    Unavailable(NoArgCallback),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SubscriptionKind {
    Connected,
    UserMessage,
    Disconnected,
    PeerConnected,
    PeerDisconnected,
    PeerLost,
    Available,
    Unavailable,
}

impl<Msg: UserMsgPayload> Event<Msg> {
    fn kind(&self) -> SubscriptionKind {
        match self {
            Self::Connected(_) => SubscriptionKind::Connected,
            Self::UserMessage(_) => SubscriptionKind::UserMessage,
            Self::Disconnected(_) => SubscriptionKind::Disconnected,
            Self::PeerConnected(_) => SubscriptionKind::PeerConnected,
            Self::PeerDisconnected(_) => SubscriptionKind::PeerDisconnected,
            Self::PeerLost(_) => SubscriptionKind::PeerLost,
            Self::Available(_) => SubscriptionKind::Available,
            Self::Unavailable(_) => SubscriptionKind::Unavailable,
        }
    }
}

fn event_kind<Msg: UserMsgPayload>(event: &EventType<Msg>) -> SubscriptionKind {
    match event {
        EventType::Connected => SubscriptionKind::Connected,
        EventType::UserMessage(_, _) => SubscriptionKind::UserMessage,
        EventType::Disconnected => SubscriptionKind::Disconnected,
        EventType::PeerConnected(_) => SubscriptionKind::PeerConnected,
        EventType::PeerDisconnected(_) => SubscriptionKind::PeerDisconnected,
        EventType::PeerDropped(_) => SubscriptionKind::PeerLost,
        EventType::Available => SubscriptionKind::Available,
        EventType::Unavailable => SubscriptionKind::Unavailable,
    }
}

pub struct RtcCallbacks<Msg: UserMsgPayload> {
    next_callback_id: u64,
    subscriptions: HashMap<u64, Event<Msg>>,
    subscriptions_by_kind: HashMap<SubscriptionKind, HashSet<u64>>,
}

impl<Msg: UserMsgPayload> Default for RtcCallbacks<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: UserMsgPayload> RtcCallbacks<Msg> {
    pub fn new() -> Self {
        Self {
            next_callback_id: 1,
            subscriptions: HashMap::new(),
            subscriptions_by_kind: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_callback_id;
        self.next_callback_id += 1;
        id
    }

    pub fn subscribe(&mut self, subscription: Event<Msg>) -> u64 {
        let id = self.next_id();
        let kind = subscription.kind();

        self.subscriptions.insert(id, subscription);
        self.subscriptions_by_kind
            .entry(kind)
            .or_default()
            .insert(id);

        id
    }

    pub fn unsubscribe(&mut self, id: u64) -> bool {
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

impl<Msg: UserMsgPayload> RtcCallbacks<Msg> {
    pub fn emit(&self, event: EventType<Msg>) -> Result<()> {
        let kind = event_kind(&event);
        let Some(ids) = self.subscriptions_by_kind.get(&kind) else {
            return Ok(());
        };

        let ids: Vec<u64> = ids.iter().copied().collect();

        for id in ids {
            let Some(subscription) = self.subscriptions.get(&id) else {
                continue;
            };
            match (subscription, &event) {
                (Event::Connected(cb), EventType::Connected) => cb.call()?,
                (Event::UserMessage(cb), EventType::UserMessage(peer, data)) => {
                    cb.call(peer, data)?
                }
                (Event::Disconnected(cb), EventType::Disconnected) => cb.call()?,
                (Event::PeerConnected(cb), EventType::PeerConnected(peer)) => cb.call(peer)?,
                (Event::PeerDisconnected(cb), EventType::PeerDisconnected(peer)) => {
                    cb.call(peer)?
                }
                (Event::PeerLost(cb), EventType::PeerDropped(peer)) => cb.call(peer)?,
                (Event::Available(cb), EventType::Available) => cb.call()?,
                (Event::Unavailable(cb), EventType::Unavailable) => cb.call()?,
                _ => {}
            }
        }

        Ok(())
    }
}
