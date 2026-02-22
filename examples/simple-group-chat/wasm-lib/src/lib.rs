use antenna::client::{AntennaEngine, EngineConfig};
use shared::{ChatClientMsg, ChatServerMsg};
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

#[wasm_bindgen]
pub struct ChatWrapper {
    engine: AntennaEngine<ChatClientMsg, ChatServerMsg>,
}

#[wasm_bindgen]
impl ChatWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, auth_token: String, room_id: String) -> Result<ChatWrapper, JsValue> {
        let config = EngineConfig {
            url,
            auth_token,
            room_id,
            ice_servers: None,
        };
        let engine = AntennaEngine::new(config)?;
        Ok(ChatWrapper { engine })
    }

    pub fn send_message(&self, text: String) {
        let msg = ChatClientMsg { text };
        self.engine.send(msg);
    }

    pub fn on_event(&self, cb: js_sys::Function) {
        self.engine.set_event_handler(cb);
    }
}
