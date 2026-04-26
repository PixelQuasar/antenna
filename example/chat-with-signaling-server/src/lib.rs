#![cfg(target_family = "wasm")]

use std::{cell::RefCell, rc::Rc};

use antenna::{
    Event, IceServerConfig, Peer, PeerID, SignalingClient, js_message, js_no_arg, js_peer,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    pub text: String,
}

#[wasm_bindgen]
pub struct ChatApp {
    peer: Rc<RefCell<Peer<Message>>>,
    signaling_client: RefCell<Option<SignalingClient>>,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new(turn_url: String) -> Result<ChatApp, JsValue> {
        console_error_panic_hook::set_once();

        let mut ice_servers = vec![IceServerConfig::new(vec![
            "stun:stun.l.google.com:19302".into(),
        ])];

        ice_servers.push(IceServerConfig::with_credentials(
            vec![turn_url],
            "user".into(),
            "password".into(),
        ));

        let peer = Peer::with_ice_servers(ice_servers);
        peer.subscribe(Event::UserMessage(
            (Self::on_message as fn(PeerID, Message)).into(),
        ));

        Ok(ChatApp {
            peer: Rc::new(RefCell::new(peer)),
            signaling_client: RefCell::new(None),
        })
    }

    #[wasm_bindgen(js_name = myId)]
    pub fn my_id(&self) -> String {
        self.peer.borrow().my_id().to_string()
    }

    #[wasm_bindgen]
    pub async fn connect(&self, ws_url: String) -> Result<(), JsValue> {
        *self.signaling_client.borrow_mut() = Some(
            SignalingClient::connect(&ws_url)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        );
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn join(&self, room_id: String) -> Result<(), JsValue> {
        let Some(client) = self.signaling_client.borrow_mut().take() else {
            return Err(JsValue::from_str("Not connected"));
        };
        client
            .join(room_id, self.peer.clone())
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn leave(&self) {
        self.peer.borrow().leave();
    }

    #[wasm_bindgen(js_name = broadcast)]
    pub fn broadcast(&self, text: String) {
        self.peer.borrow().broadcast(Message { text })
    }

    fn on_message(peer: PeerID, data: Message) {
        web_sys::console::log_2(
            &JsValue::from_str(peer.as_str()),
            &JsValue::from_str(&data.text),
        );
    }

    #[wasm_bindgen(js_name = onMessage)]
    pub fn js_on_message(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::UserMessage(js_message(cb)));
    }

    #[wasm_bindgen(js_name = onConnected)]
    pub fn js_on_connected(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::Connected(js_no_arg(cb)));
    }

    #[wasm_bindgen(js_name = onDisconnected)]
    pub fn js_on_disconnected(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::Disconnected(js_no_arg(cb)));
    }

    #[wasm_bindgen(js_name = onPeerConnected)]
    pub fn js_on_peer_connected(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::PeerConnected(js_peer(cb)));
    }

    #[wasm_bindgen(js_name = onPeerDisconnected)]
    pub fn js_on_peer_disconnected(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::PeerDisconnected(js_peer(cb)));
    }

    #[wasm_bindgen(js_name = onAvailable)]
    pub fn js_on_available(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::Available(js_no_arg(cb)));
    }

    #[wasm_bindgen(js_name = onUnavailable)]
    pub fn js_on_unavailable(&self, cb: js_sys::Function) {
        self.peer
            .borrow()
            .subscribe(Event::Unavailable(js_no_arg(cb)));
    }

    #[wasm_bindgen(js_name = connectedPeers)]
    pub fn connected_peers(&self) -> js_sys::Array {
        self.peer
            .borrow()
            .connected_peers()
            .into_iter()
            .map(|p| JsValue::from_str(&p))
            .collect()
    }
}
