mod async_callback;
mod config;
mod js_event_callback;

pub use antenna_client_shared::IceServerConfig;
pub use async_callback::async_callback;
pub(crate) use config::build_rtc_config;
pub use js_event_callback::JsEventCallback;

use wasm_bindgen::JsCast;

pub fn to_js_object<T: serde::Serialize>(
    value: &T,
) -> Result<js_sys::Object, wasm_bindgen::JsValue> {
    let js = serde_wasm_bindgen::to_value(value)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    js.dyn_into::<js_sys::Object>()
}
