use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use antenna_client_shared::CallbackId;
use antenna_protocol::{
    HandshakeInput, HandshakeMode, HandshakeStrategy, Input, MsgPayload, PeerID, SignalingPayload,
    UserMsgPayload,
};
use anyhow::{Context, Result};
use wasm_bindgen::{JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;

use crate::{Driver, IceServerConfig, JsEventCallback, Rtc, RtcCallbacks};

pub struct Peer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    driver: Rc<RefCell<Driver<Msg>>>,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    left: Rc<Cell<bool>>,
    _callback_buffer: Vec<JsEventCallback>,
}

impl<Msg> Drop for Peer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    fn drop(&mut self) {
        self.leave();
    }
}

impl<Msg> Default for Peer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Peer<Msg>
where
    Msg: UserMsgPayload + 'static,
{
    pub fn new() -> Self {
        Self::with_ice_servers(IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(ice_servers: Vec<IceServerConfig>) -> Self {
        let callbacks = Rc::new(RefCell::new(RtcCallbacks::new()));
        let driver = Rc::new(RefCell::new(Driver::new(ice_servers, callbacks.clone())));
        let left = Rc::new(Cell::new(false));

        let window = web_sys::window().expect("no global window");
        let cb = Closure::<dyn FnMut()>::new({
            let left = left.clone();
            let driver = driver.clone();
            move || {
                if left.replace(true) {
                    return;
                }
                let driver = driver.clone();
                spawn_local(async move {
                    let _ = Driver::execute(driver, Input::Leave).await;
                });
            }
        });
        let _callback_buffer = vec![JsEventCallback::new(window.into(), "beforeunload", cb)];

        Self {
            driver,
            callbacks,
            left,
            _callback_buffer,
        }
    }

    pub fn my_id(&self) -> PeerID {
        self.driver.borrow().id().clone()
    }

    pub fn subscribe(&self, subscription: Rtc<Msg>) -> CallbackId {
        self.callbacks.borrow_mut().subscribe(subscription)
    }

    pub fn unsubscribe(&self, id: CallbackId) -> bool {
        self.callbacks.borrow_mut().unsubscribe(id)
    }

    pub async fn start(&self) -> Result<String> {
        Driver::execute(self.driver.clone(), Input::InitOpenOffer).await?;

        self.driver
            .borrow()
            .metadata()
            .offer
            .clone()
            .context("Offer not found on starting")?
            .to_base64()
    }

    pub async fn receive_offer(&self, offer: &str) -> Result<String> {
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

    pub async fn receive_answer(&self, answer: &str) -> Result<()> {
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

    pub fn send(&self, peer_id: PeerID, data: Msg) {
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

    pub fn broadcast(&self, data: Msg) {
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

    pub fn leave(&self) {
        if self.left.replace(true) {
            return;
        }
        let driver = self.driver.clone();
        spawn_local(async move {
            if let Err(e) = Driver::execute(driver, Input::Leave).await {
                web_sys::console::error_1(&JsValue::from_str(&format!("{e:#}")));
            }
        });
    }

    pub fn is_connected(&self, peer_id: PeerID) -> bool {
        self.driver.borrow().is_connected(&peer_id)
    }

    pub fn connected_peers(&self) -> HashSet<String> {
        self.driver
            .borrow()
            .connected_peers()
            .iter()
            .map(|p| p.to_string())
            .collect()
    }
}
