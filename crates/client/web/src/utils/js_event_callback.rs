use std::any::Any;

use wasm_bindgen::{JsCast, closure::Closure};

pub struct JsEventCallback {
    target: web_sys::EventTarget,
    event: &'static str,
    func_ref: js_sys::Function,
    _cb: Box<dyn Any>,
}

impl Drop for JsEventCallback {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, &self.func_ref);
    }
}

impl JsEventCallback {
    pub fn new<T: ?Sized + 'static>(
        target: web_sys::EventTarget,
        event: &'static str,
        cb: Closure<T>,
    ) -> Self {
        let func_ref = cb.as_ref().unchecked_ref::<js_sys::Function>().clone();
        let _ = target.add_event_listener_with_callback(event, &func_ref);
        Self {
            target,
            event,
            func_ref,
            _cb: Box::new(cb),
        }
    }
}
