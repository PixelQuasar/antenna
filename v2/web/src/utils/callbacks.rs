use crate::utils::Msg;
use wasm_bindgen::prelude::*;

pub enum RtcEvent<T> {
    Connected,

    Message(T),

    Disconnected,
}

pub trait Dispatcher<T> {
    fn emit(&self, event: RtcEvent<T>);
}

pub type ConnectedCallback = fn();

pub type MessageCallback<T> = fn(T);

pub type DisconnectedCallback = fn();

#[derive(Clone, Default)]
pub struct RtcCallbacks<Msg> {
    pub on_connected: Option<ConnectedCallback>,

    pub js_on_connected: Option<js_sys::Function>,

    pub on_message: Option<MessageCallback<Msg>>,

    pub js_on_message: Option<js_sys::Function>,

    pub on_disconnected: Option<DisconnectedCallback>,

    pub js_on_disconnected: Option<js_sys::Function>,
}

impl Dispatcher<Msg> for RtcCallbacks<Msg> {
    fn emit(&self, event: RtcEvent<Msg>) {
        match event {
            RtcEvent::Connected => {
                if let Some(on_connected) = self.on_connected {
                    on_connected();
                }
                if let Some(on_connected) = &self.js_on_connected {
                    on_connected.call0(&JsValue::NULL).ok();
                }
            }
            RtcEvent::Message(data) => {
                if let Some(on_message) = self.on_message {
                    on_message(data.clone());
                }
                if let Some(on_message) = &self.js_on_message {
                    let arr = js_sys::Uint8Array::from(&data[..]); // FAKE SERIALIZATION
                    on_message.call1(&JsValue::NULL, &arr).ok();
                }
            }
            RtcEvent::Disconnected => {
                if let Some(on_disconnected) = self.on_disconnected {
                    on_disconnected();
                }
                if let Some(on_disconnected) = &self.js_on_disconnected {
                    on_disconnected.call0(&JsValue::NULL).ok();
                }
            }
        }
    }
}
