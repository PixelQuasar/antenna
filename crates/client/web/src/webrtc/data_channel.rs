use antenna_protocol::UserMsgPayload;
use anyhow::{Result, anyhow};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys;

pub struct DataChannelManager {
    data_channel: web_sys::RtcDataChannel,
}

impl DataChannelManager {
    pub fn new(pc: &web_sys::RtcPeerConnection, label: &str) -> Self {
        let data_channel = pc.create_data_channel(label);

        data_channel.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

        Self { data_channel }
    }

    pub fn from_existing(dc: web_sys::RtcDataChannel) -> Self {
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);
        Self { data_channel: dc }
    }

    pub fn setup_on_open<F: Fn() + 'static>(&self, on_open: F) {
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_: JsValue| {
            on_open();
        }));
        self.data_channel
            .set_onopen(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    pub fn setup_on_message<F: Fn(Vec<u8>) + 'static>(&self, on_message: F) {
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event: JsValue| {
            let event: web_sys::MessageEvent = event.unchecked_into();
            if let Ok(ab) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let bytes = js_sys::Uint8Array::new(&ab).to_vec();
                on_message(bytes);
            } else if let Some(text) = event.data().as_string() {
                on_message(text.into_bytes());
            }
        }));
        self.data_channel
            .set_onmessage(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    pub fn setup_on_close<F: Fn() + 'static>(&self, on_close: F) {
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_: JsValue| {
            on_close();
        }));
        self.data_channel
            .set_onclose(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    pub fn send_data<Msg: UserMsgPayload>(&self, data: &Msg) -> Result<()> {
        let bytes =
            serde_json::to_vec(data).map_err(|e| anyhow!("Failed to serialize message: {e}"))?;

        self.data_channel
            .send_with_u8_array(&bytes)
            .map_err(|e| anyhow!("Failed to send data: {:?}", e))
    }

    pub fn close(&self) {
        self.data_channel.close();
    }
}
