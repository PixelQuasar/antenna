use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use antenna_protocol::{ClientFSM, Input, TransportFSM};
use anyhow::Result;

use crate::{
    utils::{IceServerConfig, noop},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_transport;

pub struct Driver<T: TransportFSM + 'static> {
    /// SansIO-based protocol finite state machine to handle main logic
    fsm: Rc<RefCell<ClientFSM<T>>>,

    /// JS RTC peer connection wrapper
    pc_manager: Option<PeerConnectionManager>,

    /// JS RTC data channel wrapper
    dc_manager: Option<DataChannelManager>,

    /// ICE servers configuration
    ice_servers: Vec<IceServerConfig>,

    on_offer_ready: Rc<RefCell<js_sys::Function>>,

    on_answer_ready: Rc<RefCell<js_sys::Function>>,

    on_connected: Rc<RefCell<js_sys::Function>>,

    on_message: Rc<RefCell<js_sys::Function>>,

    on_disconnected: Rc<RefCell<js_sys::Function>>,

    on_error: Rc<RefCell<js_sys::Function>>,
}

impl<T: TransportFSM + 'static> Driver<T> {
    pub fn new(ice_servers: Vec<IceServerConfig>) -> Self {
        Self {
            fsm: Rc::new(RefCell::new(ClientFSM::new())),
            pc_manager: None,
            dc_manager: None,
            ice_servers,
            on_offer_ready: Rc::new(RefCell::new(noop())),
            on_answer_ready: Rc::new(RefCell::new(noop())),
            on_connected: Rc::new(RefCell::new(noop())),
            on_message: Rc::new(RefCell::new(noop())),
            on_disconnected: Rc::new(RefCell::new(noop())),
            on_error: Rc::new(RefCell::new(noop())),
        }
    }

    pub fn set_on_offer_ready(&mut self, cb: js_sys::Function) {
        *self.on_offer_ready.borrow_mut() = cb;
    }

    pub fn set_on_answer_ready(&mut self, cb: js_sys::Function) {
        *self.on_answer_ready.borrow_mut() = cb;
    }

    pub fn set_on_connected(&mut self, cb: js_sys::Function) {
        *self.on_connected.borrow_mut() = cb;
    }

    pub fn set_on_message(&mut self, cb: js_sys::Function) {
        *self.on_message.borrow_mut() = cb;
    }

    pub fn set_on_disconnected(&mut self, cb: js_sys::Function) {
        *self.on_disconnected.borrow_mut() = cb;
    }

    pub fn set_on_error(&mut self, cb: js_sys::Function) {
        *self.on_error.borrow_mut() = cb;
    }

    pub fn is_connected(&self) -> bool {
        self.fsm.borrow().is_connected()
    }

    pub async fn process_input(&mut self, input: Input<Vec<u8>>) -> Result<()> {
        // Feed input to FSM, execute outputs
        todo!()
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        // Send via DataChannel
        todo!()
    }
}
