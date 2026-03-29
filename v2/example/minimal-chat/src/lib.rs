use antenna::web::{Client, IceServerConfig, PeerID};
use dotenvy::dotenv;
use std::env;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ChatApp {
    client: Client,
    remote: Option<PeerID>,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> Result<ChatApp, JsValue> {
        console_error_panic_hook::set_once();

        dotenv().ok();

        let ice_servers = vec![
            IceServerConfig::new(vec!["stun:stun.l.google.com:19302".into()]),
            IceServerConfig::with_credentials(
                vec![env::var("TURN_URL").unwrap()],
                env::var("TURN_USER").unwrap(),
                env::var("TURN_PASSWORD").unwrap(),
            ),
        ];

        let mut client = Client::with_ice_servers(PeerID::new(id), ice_servers);
        client.set_on_message(Self::on_message);

        Ok(ChatApp {
            client,
            remote: None,
        })
    }

    pub async fn start_as_host(&mut self, remote_id: String) -> Result<String, JsValue> {
        self.remote = Some(PeerID::new(remote_id.clone()));

        let (offer, _relays) = self
            .client
            .start_with_peer(PeerID::new(remote_id))
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(offer)
    }

    pub async fn accept_offer(
        &mut self,
        remote_id: String,
        offer_sdp: String,
    ) -> Result<String, JsValue> {
        self.remote_peer_id = Some(remote_id.clone());

        let (answer, _relays) = self
            .client
            .receive_offer(PeerID::new(remote_id), offer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(answer)
    }

    pub async fn accept_answer(&mut self, answer_sdp: String) -> Result<(), JsValue> {
        let remote_id = self
            .remote_peer_id
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Remote peer ID not set"))?;

        self.client
            .receive_answer(remote_id, answer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(())
    }

    pub async fn send(&self, text: String) -> Result<(), JsValue> {
        let remote_id = self
            .remote_peer_id
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Not connected"))?;

        self.client
            .send_to(remote_id, text.as_bytes().to_vec())
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn is_connected(&self) -> bool {
        if let Some(remote_id) = &self.remote_peer_id {
            self.client.is_connected_to(remote_id)
        } else {
            false
        }
    }

    fn on_message(peer: PeerID, data: Vec<u8>) {
        let text = String::from_utf8_lossy(&data).to_string();
        web_sys::console::log_1(&JsValue::from_str(&text));
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
}
