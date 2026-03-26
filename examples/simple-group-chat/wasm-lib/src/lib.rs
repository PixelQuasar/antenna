use antenna::client::{antenna_client, AntennaEngine, EngineConfig};
use shared::{ChatClientMsg, ChatServerMsg};
use wasm_bindgen::prelude::*;
use web_sys::js_sys;


struct ChatMsg {
    author: u64;
    text: String;
    timestamp: u64;
}

#[antenna_client(ChatClientMsg)]
#[wasm_bindgen]
pub struct ChatWrapper {
    engine: AntennaEngine<ChatClientMsg>,
}

#[wasm_bindgen]
impl ChatWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, room_id: String) -> Result<ChatWrapper, JsValue> {
        let config = EngineConfig {
            url,
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
}
