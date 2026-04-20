use antenna::web::{IceServerConfig, Peer, PeerID, Rtc};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    pub text: String,
}

#[wasm_bindgen]
pub struct ChatApp {
    peer: Peer<Message>,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new(turn_url: String) -> Result<ChatApp, JsValue> {
        console_error_panic_hook::set_once();

        let mut ice_servers = vec![IceServerConfig::new(vec![
            "stun:stun.l.google.com:19302".into(),
        ])];

        let user = "user";
        let pass = "password";

        ice_servers.push(IceServerConfig::with_credentials(
            vec![turn_url],
            user.into(),
            pass.into(),
        ));

        let mut peer = Peer::with_ice_servers(ice_servers);
        peer.subscribe(Rtc::UserMessage(Self::on_message));

        Ok(ChatApp { peer })
    }

    #[wasm_bindgen(js_name = startAsHost)]
    pub async fn start_as_host(&mut self, remote_id: String) -> Result<String, JsValue> {
        self.peer
            .start(PeerID::new(remote_id))
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = acceptOffer)]
    pub async fn accept_offer(
        &mut self,
        remote_id: String,
        offer_sdp: String,
    ) -> Result<String, JsValue> {
        self.peer
            .receive_offer(PeerID::new(remote_id), offer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = acceptAnswer)]
    pub async fn accept_answer(
        &mut self,
        remote_id: String,
        answer_sdp: String,
    ) -> Result<(), JsValue> {
        self.peer
            .receive_answer(PeerID::new(remote_id), answer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = broadcast)]
    pub async fn broadcast(&mut self, text: String) -> Result<(), JsValue> {
        self.peer
            .broadcast(Message { text })
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
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
}
