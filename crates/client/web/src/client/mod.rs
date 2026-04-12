use std::{cell::RefCell, rc::Rc};

use crate::{
    driver::Driver,
    utils::{CallbackId, IceServerConfig, Rtc, RtcCallbacks},
};
use antenna_protocol::{
    HandshakeInput, Input, MsgPayload, PeerID, SignalingPayload, UserMsgPayload,
};

use anyhow::{Context, Result};

pub struct Client<Msg>
where
    Msg: UserMsgPayload,
{
    my_id: PeerID,
    driver: Rc<RefCell<Driver<Msg>>>,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl<Msg> Client<Msg>
where
    Msg: UserMsgPayload,
{
    pub fn new(my_id: PeerID) -> Self {
        Self::with_ice_servers(my_id, IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(my_id: PeerID, ice_servers: Vec<IceServerConfig>) -> Self {
        let callbacks = Rc::new(RefCell::new(RtcCallbacks::new()));
        let driver = Rc::new(RefCell::new(Driver::new(
            my_id.clone(),
            ice_servers,
            callbacks.clone(),
        )));
        driver.borrow_mut().attach_self(driver.clone());

        Self {
            my_id,
            driver,
            callbacks,
        }
    }

    pub fn my_id(&self) -> &PeerID {
        &self.my_id
    }

    pub fn subscribe(&mut self, subscription: Rtc<Msg>) -> CallbackId {
        self.callbacks.borrow_mut().subscribe(subscription)
    }

    pub fn unsubscribe(&mut self, id: CallbackId) -> bool {
        self.callbacks.borrow_mut().unsubscribe(id)
    }

    pub async fn start_bootstrap(&mut self, peer_id: PeerID) -> Result<String> {
        {
            let mut driver = self.driver.borrow_mut();
            driver.init_host(peer_id.clone()).await?;
            driver
                .execute(Input::Handshake {
                    from: peer_id.clone(),
                    event: HandshakeInput::Init,
                })
                .await?;
        }

        self.driver
            .borrow()
            .fsm()
            .borrow()
            .metadata()
            .sdp_offer
            .clone()
            .context("SDP offer not found on starting")
    }

    pub async fn receive_bootstrap_offer(
        &mut self,
        peer_id: PeerID,
        offer: String,
    ) -> Result<String> {
        {
            let mut driver = self.driver.borrow_mut();
            driver.init_joiner(peer_id.clone()).await?;
            driver
                .execute(Input::Handshake {
                    from: peer_id.clone(),
                    event: HandshakeInput::Signaling(SignalingPayload::Offer(offer)),
                })
                .await?;
        }

        self.driver
            .borrow()
            .fsm()
            .borrow()
            .metadata()
            .sdp_answer
            .clone()
            .context("SDP answer not found on receiving offer")
    }

    pub async fn receive_answer(&mut self, peer_id: PeerID, answer: String) -> Result<()> {
        self.driver
            .borrow_mut()
            .execute(Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Signaling(SignalingPayload::Answer(answer)),
            })
            .await?;

        Ok(())
    }

    pub async fn send(&mut self, peer_id: PeerID, data: Msg) -> Result<()> {
        self.driver
            .borrow_mut()
            .execute(Input::Send {
                peer_to: peer_id,
                data: MsgPayload::User(data),
            })
            .await?;
        Ok(())
    }

    pub async fn broadcast(&mut self, data: Msg) -> Result<()> {
        self.driver
            .borrow_mut()
            .execute(Input::Broadcast {
                data: MsgPayload::User(data),
            })
            .await?;
        Ok(())
    }

    pub fn is_connected(&self, peer_id: PeerID) -> bool {
        self.driver.borrow().is_connected(&peer_id)
    }

    pub fn connected_peers(&self) -> Vec<String> {
        self.driver
            .borrow()
            .connected_peers()
            .into_iter()
            .map(|p| p.to_string())
            .collect()
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
}
