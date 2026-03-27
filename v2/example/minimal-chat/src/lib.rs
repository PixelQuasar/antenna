use antenna::web::{Client, IceServerConfig};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ChatApp {
    client: Client,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<ChatApp, JsValue> {
        console_error_panic_hook::set_once();

        let ice_servers = vec![
            IceServerConfig::new(vec!["stun:stun.l.google.com:19302".into()]),
            IceServerConfig::with_credentials(
                vec!["turn:demo-voicechat.quasarity.com:3478".into()],
                "user".into(),
                "password".into(),
            ),
        ];

        let mut client = Client::with_ice_servers(ice_servers);
        client.set_on_message(Self::on_message);

        Ok(ChatApp { client })
    }

    pub async fn start_as_host(&mut self) -> Result<String, JsValue> {
        self.client
            .start()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub async fn accept_offer(&mut self, offer_sdp: String) -> Result<String, JsValue> {
        self.client
            .receive_offer(offer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub async fn accept_answer(&mut self, answer_sdp: String) -> Result<(), JsValue> {
        self.client
            .receive_answer(answer_sdp)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub async fn send(&self, text: String) -> Result<(), JsValue> {
        self.client
            .send(text.as_bytes())
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    fn on_message(data: Vec<u8>) {
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
