use std::{cell::RefCell, rc::Rc};

use crate::{
    driver::Driver,
    utils::{
        IceServerConfig, MessageCallback, PeerConnectedCallback, PeerDisconnectedCallback,
        RtcCallbacks,
    },
};
use antenna_protocol::{UserMsgPayload, HandshakeInput, Input, PeerID};

use anyhow::{Context, Result};

pub struct Client<Msg>
where
    Msg: UserMsgPayload,
{
    my_id: PeerID,
    driver: Driver<Msg>,
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
        let driver = Driver::new(my_id.clone(), ice_servers, callbacks.clone());

        Self {
            my_id,
            driver,
            callbacks,
        }
    }

    pub fn my_id(&self) -> &PeerID {
        &self.my_id
    }

    pub async fn start(&mut self, peer_id: PeerID) -> Result<String> {
        self.driver
            .process_input(Input::Handshake {
                from: peer_id.clone(),
                event: HandshakeInput::InitNegotiation,
            })
            .await?;
        self.driver
            .fsm()
            .borrow()
            .metadata()
            .sdp_offer
            .clone()
            .context("SDP offer not found on starting")
    }

    pub async fn receive_offer(&mut self, peer_id: PeerID, offer: String) -> Result<String> {
        self.driver
            .process_input(Input::Handshake {
                from: peer_id.clone(),
                event: HandshakeInput::SDPOfferReceived { sdp: offer },
            })
            .await?;
        self.driver
            .fsm()
            .borrow()
            .metadata()
            .sdp_answer
            .clone()
            .context("SDP answer not found on receiving offer")
    }

    pub async fn receive_answer(&mut self, peer_id: PeerID, answer: String) -> Result<()> {
        self.driver
            .process_input(Input::Handshake {
                from: peer_id,
                event: HandshakeInput::SDPAnswerReceived { sdp: answer },
            })
            .await?;

        Ok(())
    }

    pub async fn send_to(&mut self, peer_id: PeerID, data: Msg) -> Result<()> {
        self.driver
            .process_input(Input::PeerSend {
                peer_to: peer_id,
                data: data,
            })
            .await?;
        Ok(())
    }

    pub async fn broadcast(&mut self, data: Msg) -> Result<()> {
        self.driver
            .process_input(Input::PeerBroadcast { data: data })
            .await?;
        Ok(())
    }

    pub fn is_connected(&self, peer_id: PeerID) -> bool {
        self.driver.is_connected(&peer_id)
    }

    pub fn connected_peers(&self) -> Vec<String> {
        self.driver
            .connected_peers()
            .into_iter()
            .map(|p| p.to_string())
            .collect()
    }

    pub fn set_on_message(&mut self, cb: MessageCallback<Msg>) {
        self.callbacks.borrow_mut().on_message = Some(cb);
    }

    pub fn set_on_peer_connected(&mut self, cb: PeerConnectedCallback) {
        self.callbacks.borrow_mut().on_peer_connected = Some(cb);
    }

    pub fn set_on_peer_disconnected(&mut self, cb: PeerDisconnectedCallback) {
        self.callbacks.borrow_mut().on_peer_disconnected = Some(cb);
    }

    pub fn set_js_on_message(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_message = Some(cb)
    }

    pub fn set_js_on_connected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_connected = Some(cb)
    }

    pub fn set_js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_disconnected = Some(cb)
    }
}
