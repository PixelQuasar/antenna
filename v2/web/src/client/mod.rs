mod connection;

use std::{cell::RefCell, rc::Rc};

use crate::{
    driver::Driver,
    utils::{
        ConnectedCallback, DisconnectedCallback, IceServerConfig, MessageCallback, Msg,
        RtcCallbacks,
    },
};
use antenna_protocol::{Host, Input, Joiner, TransportInput};
use anyhow::Result;
use connection::Connection;
use wasm_bindgen::JsValue;

pub struct Client {
    connection: Connection,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl Client {
    pub fn new() -> Self {
        Self::with_ice_servers(IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(ice_servers: Vec<IceServerConfig>) -> Self {
        Self {
            connection: Connection::Connecting { ice_servers },
            callbacks: Rc::new(RefCell::new(RtcCallbacks::default())),
        }
    }

    pub async fn start(&mut self) -> Result<String> {
        let ice_servers = match &self.connection {
            Connection::Connecting { ice_servers } => ice_servers.clone(),
            _ => return Err(anyhow::anyhow!("Connection already initialized")),
        };
        let mut driver = Driver::<Host>::new(ice_servers, self.callbacks.clone());

        driver
            .process_input(Input::Transport(TransportInput::InitNegotiation))
            .await?;

        let sdp = driver
            .local_sdp()
            .ok_or_else(|| anyhow::anyhow!("No SDP generated"))?
            .to_string();

        self.connection = Connection::Host(driver);
        Ok(sdp)
    }

    pub async fn receive_offer(&mut self, offer_sdp: String) -> Result<String> {
        let ice_servers = match &self.connection {
            Connection::Connecting { ice_servers } => ice_servers.clone(),
            _ => return Err(anyhow::anyhow!("Connection already initialized")),
        };
        let mut driver = Driver::<Joiner>::new(ice_servers, self.callbacks.clone());
        driver
            .process_input(Input::Transport(TransportInput::SDPOfferReceived {
                sdp: offer_sdp,
            }))
            .await?;
        let sdp = driver
            .local_sdp()
            .ok_or_else(|| anyhow::anyhow!("No SDP generated"))?;

        self.connection = Connection::Joiner(driver);
        Ok(sdp)
    }

    pub async fn receive_answer(&mut self, answer_sdp: String) -> Result<()> {
        match &mut self.connection {
            Connection::Host(driver) => {
                driver
                    .process_input(Input::Transport(TransportInput::SDPAnswerReceived {
                        sdp: answer_sdp,
                    }))
                    .await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!(
                "receive_answer can only be called on Host connection"
            )),
        }
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        match &self.connection {
            Connection::Host(driver) => driver.send(data).await,
            Connection::Joiner(driver) => driver.send(data).await,
            Connection::Connecting { .. } => {
                Err(anyhow::anyhow!("Cannot send: connection not initialized"))
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        match &self.connection {
            Connection::Host(driver) => driver.is_connected(),
            Connection::Joiner(driver) => driver.is_connected(),
            Connection::Connecting { .. } => false,
        }
    }

    pub fn set_on_connected(&mut self, cb: ConnectedCallback) {
        self.callbacks.borrow_mut().on_connected = Some(cb);
    }

    pub fn set_on_message(&mut self, cb: MessageCallback<Msg>) {
        self.callbacks.borrow_mut().on_message = Some(cb);
    }

    pub fn set_on_disconnected(&mut self, cb: DisconnectedCallback) {
        self.callbacks.borrow_mut().on_disconnected = Some(cb);
    }

    pub fn set_js_on_connected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_connected = Some(cb)
    }

    pub fn set_js_on_message(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_message = Some(cb)
    }

    pub fn set_js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_disconnected = Some(cb)
    }
}
