use antenna::web::{Client, IceServerConfig, PeerID, Rtc};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    pub text: String,
}

#[wasm_bindgen]
pub struct ChatApp {
    client: Client<Message>,
    remote: Option<PeerID>,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, turn_url: String) -> Result<ChatApp, JsValue> {
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

        let mut client = Client::with_ice_servers(PeerID::new(id), ice_servers);
        client.subscribe(Rtc::UserMessage(Self::on_message));

        Ok(ChatApp {
            client,
            remote: None,
        })
    }

    pub async fn start_as_host(&mut self, remote_id: String) -> Result<String, JsValue> {
        self.remote = Some(PeerID::new(remote_id.clone()));
        let offer = self
            .client
            .start_bootstrap(PeerID::new(remote_id))
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(offer)
    }

    pub async fn accept_offer(
        &mut self,
        remote_id: String,
        offer_sdp: String,
    ) -> Result<String, JsValue> {
        self.remote = Some(PeerID::new(remote_id.clone()));
        let answer = self
            .client
            .receive_bootstrap_offer(PeerID::new(remote_id), offer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(answer)
    }

    pub async fn accept_answer(&mut self, answer_sdp: String) -> Result<(), JsValue> {
        let remote_id = self
            .remote
            .clone()
            .ok_or_else(|| JsValue::from_str("Remote peer ID not set"))?;
        self.client
            .receive_answer(remote_id, answer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(())
    }

    pub async fn send(&mut self, text: String) -> Result<(), JsValue> {
        let remote_id = self
            .remote
            .clone()
            .ok_or_else(|| JsValue::from_str("Not connected"))?;
        self.client
            .send(remote_id, Message { text })
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn is_connected(&self) -> bool {
        if let Some(remote_id) = &self.remote {
            self.client.is_connected(remote_id.clone())
        } else {
            false
        }
    }

    #[wasm_bindgen(js_name = connectedPeers)]
    pub fn connected_peers(&self) -> js_sys::Array {
        self.client
            .connected_peers()
            .into_iter()
            .map(JsValue::from)
            .collect()
    }

    fn on_message(peer: PeerID, data: Message) {
        web_sys::console::log_2(
            &JsValue::from_str(&peer.as_str()),
            &JsValue::from_str(&data.text),
        );
    }

    #[wasm_bindgen(js_name = onMessage)]
    pub fn js_on_message(&mut self, cb: js_sys::Function) {
        self.client.set_js_on_message(cb);
    }

    #[wasm_bindgen(js_name = onConnected)]
    pub fn js_on_connected(&mut self, cb: js_sys::Function) {
        self.client.set_js_on_connected(cb);
    }

    #[wasm_bindgen(js_name = onDisconnected)]
    pub fn js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.client.set_js_on_disconnected(cb);
    }

    #[wasm_bindgen(js_name = onPeerConnected)]
    pub fn js_on_peer_connected(&mut self, cb: js_sys::Function) {
        self.client.set_js_on_peer_connected(cb);
    }

    #[wasm_bindgen(js_name = onPeerDisconnected)]
    pub fn js_on_peer_disconnected(&mut self, cb: js_sys::Function) {
        self.client.set_js_on_peer_disconnected(cb);
    }
}
