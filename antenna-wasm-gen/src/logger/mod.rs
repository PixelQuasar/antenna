use wasm_bindgen::JsValue;
use web_sys::console;

pub struct Logger;

impl Logger {
    pub fn info(msg: &str) {
        console::log_1(&format!("[INFO] {}", msg).into());
    }

    pub fn warn(msg: &str) {
        console::log_1(&format!("[WARN] {}", msg).into());
    }

    pub fn error(err: &JsValue) {
        console::log_2(&format!("[ERROR]").into(), err);
    }

    pub fn _debug(msg: &str) {
        console::log_1(&format!("[DEBUG] {}", msg).into());
    }
}
