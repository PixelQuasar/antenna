mod async_callback;
mod config;

pub use async_callback::async_callback;
pub use config::IceServerConfig;
use wasm_bindgen::JsCast;

pub fn to_js_object<T: serde::Serialize>(
    value: &T,
) -> Result<js_sys::Object, wasm_bindgen::JsValue> {
    let js = serde_wasm_bindgen::to_value(value)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;

    js.dyn_into::<js_sys::Object>()
}

pub const STORAGE_IDENTITY_KEY: &str = "antenna_identity";

pub const EXECUTE_FUEL: u64 = 1024;
