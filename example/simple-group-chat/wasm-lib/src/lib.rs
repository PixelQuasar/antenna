use antenna::engine::{AntennaEngine, EngineConfig};
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

#[wasm_bindgen]
pub struct ChatClient {
    engine: AntennaEngine<ChatInput, ChatEvent>,
}

#[wasm_bindgen]
impl ChatClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, user: String) -> Self {
        let config = EngineConfig {
            url,
            auth_token: user,
        };
        Self {
            engine: AntennaEngine::new(config).unwrap(),
        }
    }

    pub fn send_text(&self, text: String) {
        self.engine.send(ChatInput::SendMessage { text });
    }

    pub fn on_event(&self, cb: js_sys::Function) {
        self.engine.set_event_handler(cb);
    }
}
