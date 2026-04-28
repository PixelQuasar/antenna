use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use crate::{Driver, JsEventCallback, Storage};
use antenna_client_shared::{Event, IceServerConfig, RtcCallbacks, STORAGE_IDENTITY_KEY};
use antenna_protocol::{PeerID, UserMsgPayload};
use anyhow::Result;
use wasm_bindgen::closure::Closure;

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
        Self::with_storage(ice_servers, Storage::new(STORAGE_IDENTITY_KEY))
    }

    pub fn with_storage(ice_servers: Vec<IceServerConfig>, storage: Storage) -> Self {
        let callbacks = Rc::new(RefCell::new(RtcCallbacks::new()));
        let driver = Rc::new(RefCell::new(Driver::new(
            ice_servers,
            callbacks.clone(),
            storage,
        )));
        let left = Rc::new(Cell::new(false));

        let window = web_sys::window().expect("no global window");
        let cb = Closure::<dyn FnMut()>::new({
            let left = left.clone();
            let driver = driver.clone();
            move || {
                if left.replace(true) {
                    return;
                }
                Driver::leave(driver.clone());
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
        *self.driver.borrow().id()
    }

    pub fn subscribe(&self, subscription: Event<Msg>) -> u64 {
        self.callbacks.borrow_mut().subscribe(subscription)
    }

    pub fn unsubscribe(&self, id: u64) -> bool {
        self.callbacks.borrow_mut().unsubscribe(id)
    }

    pub async fn start(&self) -> Result<String> {
        Driver::start(self.driver.clone()).await
    }

    pub async fn receive_offer(&self, offer: &str) -> Result<String> {
        Driver::receive_offer(self.driver.clone(), offer).await
    }

    pub async fn receive_answer(&self, answer: &str) -> Result<()> {
        Driver::receive_answer(self.driver.clone(), answer).await
    }

    pub fn send(&self, peer_id: PeerID, data: Msg) {
        Driver::send(self.driver.clone(), peer_id, data);
    }

    pub fn broadcast(&self, data: Msg) {
        Driver::broadcast(self.driver.clone(), data);
    }

    pub fn leave(&self) {
        if self.left.replace(true) {
            return;
        }
        Driver::leave(self.driver.clone());
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
