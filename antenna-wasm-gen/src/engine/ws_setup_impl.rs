use antenna_core::{Message, SignalMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsValue, prelude::Closure};
use web_sys::WebSocket;

use crate::AntennaEngine;
use crate::{ConnectionState, EngineConfig, logger::Logger};

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(crate) fn ws_setup(&self, config: EngineConfig) -> Result<(), JsValue> {
        let ws: WebSocket = web_sys::WebSocket::new(&config.url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let onopen_callback = {
            let service = self.service.clone();
            let token = config.auth_token.clone();
            Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
                Logger::info(&"WS Open");

                let join_msg = SignalMessage::Join {
                    room: "DEFAULT".to_string(),
                    token: Some(token.clone()),
                };

                let json = serde_json::to_string(&join_msg).unwrap();
                if let Some(ws) = &service.borrow().ws {
                    ws.send_with_str(&json).unwrap();
                }
                service.borrow_mut().state = ConnectionState::Connecting;
            }))
        };
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let onmessage_callback = {
            let service = self.service.clone();
            Closure::<dyn FnMut(web_sys::MessageEvent)>::wrap(Box::new(
                move |e: web_sys::MessageEvent| {
                    if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                        let text: String = text.into();
                        Logger::info(&format!("WS IN: {}", text));
                        Self::handle_signal(&service, text);
                    }
                },
            ))
        };
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        self.service.borrow_mut().ws = Some(ws);
        Ok(())
    }
}
