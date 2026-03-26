mod async_callback;
mod config;

pub use async_callback::async_callback;
pub use config::IceServerConfig;

use anyhow::Error;
use wasm_bindgen::prelude::*;

pub fn to_js_error(err: Error) -> JsValue {
    JsValue::from_str(&format!("{:#}", err))
}

pub fn noop() -> js_sys::Function {
    Closure::<dyn Fn()>::new(|| {})
        .into_js_value()
        .unchecked_into()
}
