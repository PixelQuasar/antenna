use antenna_web_client::MeshNode;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub author: String,
    pub text: String,
    pub timestamp: u64,
}

#[wasm_bindgen]
pub struct ChatWrapper {
    node: MeshNode,
}

#[wasm_bindgen]
impl ChatWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<ChatWrapper, JsValue> {
        let node = MeshNode::new_master(vec![]);
        Ok(ChatWrapper { node })
    }

    #[wasm_bindgen(js_name = "join")]
    pub fn join() -> Result<ChatWrapper, JsValue> {
        let node = MeshNode::new_joiner();
        Ok(ChatWrapper { node })
    }

    #[wasm_bindgen(js_name = "sendMessage")]
    pub fn send_message(&self, text: String) -> Result<(), JsValue> {
        let msg = ChatMsg {
            author: "anon".to_string(),
            text,
            timestamp: 0,
        };
        self.node.broadcast_json(&msg)
    }

    #[wasm_bindgen(js_name = "onMessage")]
    pub fn on_message(&self, cb: js_sys::Function) {
        self.node.on_json_message(move |_peer_id, msg: ChatMsg| {
            if let Ok(js_val) = serde_wasm_bindgen::to_value(&msg) {
                let _ = cb.call1(&JsValue::NULL, &js_val);
            }
        });
    }

    // -- Bootstrap ----------------------------------------------------------

    #[wasm_bindgen(js_name = "generateInvite")]
    pub fn generate_invite(&self) -> Result<(), JsValue> {
        self.node.generate_invite()
    }

    #[wasm_bindgen(js_name = "onInviteReady")]
    pub fn on_invite_ready(&self, cb: js_sys::Function) {
        self.node.on_invite_ready(move |json| {
            let _ = cb.call1(&JsValue::NULL, &JsValue::from_str(&json));
        });
    }

    #[wasm_bindgen(js_name = "onAnswerReady")]
    pub fn on_answer_ready(&self, cb: js_sys::Function) {
        self.node.on_answer_ready(move |json| {
            let _ = cb.call1(&JsValue::NULL, &JsValue::from_str(&json));
        });
    }

    #[wasm_bindgen(js_name = "acceptAnswer")]
    pub fn accept_answer(&self, json: &str) -> Result<(), JsValue> {
        self.node.accept_answer(json)
    }

    #[wasm_bindgen(js_name = "acceptInvite")]
    pub fn accept_invite(&self, json: &str) -> Result<(), JsValue> {
        self.node.accept_invite(json)
    }
}
