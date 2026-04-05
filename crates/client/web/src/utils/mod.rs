mod async_callback;
mod callbacks;
mod config;

pub use async_callback::async_callback;
pub use callbacks::{CallbackId, Dispatcher, Rtc, RtcCallbacks, RtcEvent};
pub use config::IceServerConfig;
use wasm_bindgen::JsCast;

fn to_js_object<T: serde::Serialize>(value: &T) -> Result<js_sys::Object, wasm_bindgen::JsValue> {
    let js = serde_wasm_bindgen::to_value(value)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;

    js.dyn_into::<js_sys::Object>()
}
