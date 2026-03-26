use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use antenna_protocol::{ClientFSM, Input, Output, TransportFSM};
use anyhow::{Context, Result};
use wasm_bindgen::prelude::*;

use crate::{
    utils::{IceServerConfig, noop},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_transport;

pub type Msg = Vec<u8>; // TODO REMOVE LATER! hardcode

pub type OfferReadyCallback = fn(String);

pub type AnswerReadyCallback = fn(String);

pub type ConnectedCallback = fn();

pub type MessageCallback<T> = fn(T);

pub type DisconnectedCallback = fn();

pub type ErrorCallback = fn(String);

pub struct Driver<T: TransportFSM + 'static> {
    /// SansIO-based protocol finite state machine to handle main logic
    fsm: Rc<RefCell<ClientFSM<T>>>,

    /// JS RTC peer connection wrapper
    pc_manager: Option<PeerConnectionManager>,

    /// JS RTC data channel wrapper
    dc_manager: Option<DataChannelManager>,

    /// ICE servers configuration
    ice_servers: Vec<IceServerConfig>,

    on_offer_ready: Option<OfferReadyCallback>,

    on_answer_ready: Option<AnswerReadyCallback>,

    on_connected: Option<ConnectedCallback>,

    on_message: Option<MessageCallback<Msg>>,

    on_disconnected: Option<DisconnectedCallback>,

    on_error: Option<ErrorCallback>,
}

impl<T: TransportFSM + 'static> Driver<T> {
    pub fn new(ice_servers: Vec<IceServerConfig>) -> Self {
        Self {
            fsm: Rc::new(RefCell::new(ClientFSM::new())),
            pc_manager: None,
            dc_manager: None,
            ice_servers,
            on_offer_ready: None,
            on_answer_ready: None,
            on_connected: None,
            on_message: None,
            on_disconnected: None,
            on_error: None,
        }
    }

    pub fn set_on_offer_ready(&mut self, cb: OfferReadyCallback) {
        self.on_offer_ready = Some(cb);
    }

    pub fn set_on_answer_ready(&mut self, cb: AnswerReadyCallback) {
        self.on_answer_ready = Some(cb);
    }

    pub fn set_on_connected(&mut self, cb: ConnectedCallback) {
        self.on_connected = Some(cb);
    }

    pub fn set_on_message(&mut self, cb: MessageCallback<Msg>) {
        self.on_message = Some(cb);
    }

    pub fn set_on_disconnected(&mut self, cb: DisconnectedCallback) {
        self.on_disconnected = Some(cb);
    }

    pub fn set_on_error(&mut self, cb: ErrorCallback) {
        self.on_error = Some(cb);
    }

    pub fn is_connected(&self) -> bool {
        self.fsm.borrow().is_connected()
    }

    pub async fn process_input(&mut self, input: Input<Msg>) -> Result<()> {
        let output = self.fsm.borrow_mut().process(input);

        if let Some(output) = output {
            match output {
                Output::Transport(transport_output) => {
                    self.execute_transport::<Msg>(transport_output).await?;
                }
                Output::SendMessage { data, .. } | Output::Broadcast { data } => {
                    self.send(&data).await?;
                }
                Output::ReceiveMessage { data, .. } => {
                    if let Some(on_message) = self.on_message {
                        let _ = on_message(data);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let dc_manager = self
            .dc_manager
            .as_ref()
            .context("DataChannel not initialized")?;

        dc_manager.send_data(data)?;
        Ok(())
    }
}
