use std::{cell::RefCell, collections::HashSet, rc::Rc};

use antenna_client_shared::{CallbackId, Peer};
use antenna_protocol::{
    HandshakeInput, HandshakeMode, HandshakeStrategy, Input, MsgPayload, PeerID, SignalingPayload,
    UserMsgPayload,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{Driver, IceServerConfig, Rtc, RtcCallbacks};

pub struct WebPeer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    driver: Rc<RefCell<Driver<Msg>>>,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl<Msg> Clone for WebPeer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    fn clone(&self) -> Self {
        Self {
            driver: self.driver.clone(),
            callbacks: self.callbacks.clone(),
        }
    }
}

impl<Msg> Default for WebPeer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> WebPeer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    pub fn new() -> Self {
        Self::with_ice_servers(IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(ice_servers: Vec<IceServerConfig>) -> Self {
        let callbacks = Rc::new(RefCell::new(RtcCallbacks::new()));
        let driver = Rc::new(RefCell::new(Driver::new(ice_servers, callbacks.clone())));
        Self { driver, callbacks }
    }

    pub fn set_js_on_message(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsUserMessage(cb));
    }

    pub fn set_js_on_connected(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsConnected(cb));
    }

    pub fn set_js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsDisconnected(cb));
    }

    pub fn set_js_on_peer_connected(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsPeerConnected(cb));
    }

    pub fn set_js_on_peer_disconnected(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsPeerDisconnected(cb));
    }

    pub fn set_js_on_available(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsAvailable(cb));
    }

    pub fn set_js_on_unavailable(&mut self, cb: js_sys::Function) {
        self.subscribe(Rtc::JsUnavailable(cb));
    }
}

#[async_trait(?Send)]
impl<Msg> Peer<Msg> for WebPeer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    type Subscription = Rtc<Msg>;

    fn my_id(&self) -> PeerID {
        self.driver.borrow().id().clone()
    }

    fn subscribe(&mut self, subscription: Rtc<Msg>) -> CallbackId {
        self.callbacks.borrow_mut().subscribe(subscription)
    }

    fn unsubscribe(&mut self, id: CallbackId) -> bool {
        self.callbacks.borrow_mut().unsubscribe(id)
    }

    async fn start(&self) -> Result<String> {
        Driver::execute(self.driver.clone(), Input::InitOpenOffer).await?;

        self.driver
            .borrow()
            .metadata()
            .offer
            .clone()
            .context("Offer not found on starting")?
            .to_base64()
    }

    async fn receive_offer(&self, offer: &str) -> Result<String> {
        let offer = SignalingPayload::from_base64(offer)?;
        let peer_id = offer.peer_id();
        Driver::execute(
            self.driver.clone(),
            Input::InitHandshake {
                with: peer_id.clone(),
                mode: HandshakeMode::Bootstrap,
                strategy: HandshakeStrategy::Joiner,
            },
        )
        .await?;
        Driver::execute(
            self.driver.clone(),
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Offer(offer),
            },
        )
        .await?;

        self.driver
            .borrow()
            .metadata()
            .answer
            .clone()
            .context("Answer not found on receiving offer")?
            .to_base64()
    }

    async fn receive_answer(&self, answer: &str) -> Result<()> {
        let answer = SignalingPayload::from_base64(answer)?;
        let peer_id = answer.peer_id();
        Driver::execute(
            self.driver.clone(),
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Answer(answer),
            },
        )
        .await?;

        Ok(())
    }

    fn send(&self, peer_id: PeerID, data: Msg) {
        let driver = self.driver.clone();
        spawn_local(async move {
            if let Err(e) = Driver::execute(
                driver,
                Input::Send {
                    peer_to: peer_id,
                    data: MsgPayload::User(data),
                },
            )
            .await
            {
                web_sys::console::error_1(&JsValue::from_str(&format!("{e:#}")));
            }
        });
    }

    fn broadcast(&self, data: Msg) {
        let driver = self.driver.clone();
        spawn_local(async move {
            if let Err(e) = Driver::execute(
                driver,
                Input::Broadcast {
                    data: MsgPayload::User(data),
                },
            )
            .await
            {
                web_sys::console::error_1(&JsValue::from_str(&format!("{e:#}")));
            }
        });
    }

    fn is_connected(&self, peer_id: PeerID) -> bool {
        self.driver.borrow().is_connected(&peer_id)
    }

    fn connected_peers(&self) -> HashSet<String> {
        self.driver
            .borrow()
            .connected_peers()
            .iter()
            .map(|p| p.to_string())
            .collect()
    }
}
