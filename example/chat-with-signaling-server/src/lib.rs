use std::cell::RefCell;

use antenna::{IceServerConfig, Peer, PeerID, Rtc, SignalingClient};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    pub text: String,
}

#[wasm_bindgen]
pub struct ChatApp {
    peer: Peer<Message>,
    signaling: RefCell<Option<SignalingClient>>,
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

        let mut peer = Peer::with_ice_servers(ice_servers);
        peer.subscribe(Rtc::UserMessage(Self::on_message));

        Ok(ChatApp {
            peer,
            signaling: RefCell::new(None),
        })
    }

    #[wasm_bindgen(js_name = myId)]
    pub fn my_id(&self) -> String {
        self.peer.my_id().to_string()
    }

    #[wasm_bindgen(js_name = join)]
    pub async fn join_room(&self, ws_url: String, room_id: String) -> Result<(), JsValue> {
        let mut client = SignalingClient::connect(&ws_url)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        client
            .join(&room_id, &self.peer)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        *self.signaling.borrow_mut() = Some(client);
        Ok(())
    }

    #[wasm_bindgen(js_name = broadcast)]
    pub fn broadcast(&self, text: String) {
        self.peer.broadcast(Message { text })
    }

    fn on_message(peer: PeerID, data: Message) {
        web_sys::console::log_2(
            &JsValue::from_str(&peer.as_str()),
            &JsValue::from_str(&data.text),
        );
    }

    #[wasm_bindgen(js_name = onMessage)]
    pub fn js_on_message(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_message(cb);
    }

    #[wasm_bindgen(js_name = onConnected)]
    pub fn js_on_connected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_connected(cb);
    }

    #[wasm_bindgen(js_name = onDisconnected)]
    pub fn js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_disconnected(cb);
    }

    #[wasm_bindgen(js_name = onPeerConnected)]
    pub fn js_on_peer_connected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_peer_connected(cb);
    }

    #[wasm_bindgen(js_name = onPeerDisconnected)]
    pub fn js_on_peer_disconnected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_peer_disconnected(cb);
    }

    #[wasm_bindgen(js_name = onAvailable)]
    pub fn js_on_available(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_available(cb);
    }

    #[wasm_bindgen(js_name = connectedPeers)]
    pub fn connected_peers(&self) -> js_sys::Array {
        self.peer
            .connected_peers()
            .into_iter()
            .map(|p| JsValue::from_str(&p))
            .collect()
    }
}
