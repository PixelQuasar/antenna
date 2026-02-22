use crate::AntennaEngine;
use crate::ConnectionState;
use crate::engine::EngineInner;
use crate::logger::Logger;
use antenna_core::Message;
use antenna_core::Packet;

use postcard::from_bytes;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(super) fn setup_data_channel(
        inner: &Rc<RefCell<EngineInner>>,
        dc: web_sys::RtcDataChannel,
    ) {
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

        let on_msg = {
            let inner = inner.clone();
            Closure::<dyn FnMut(web_sys::MessageEvent)>::wrap(Box::new(
                move |ev: web_sys::MessageEvent| {
                    if let Ok(ab) = ev.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let bytes = js_sys::Uint8Array::new(&ab).to_vec();
                        if let Ok(packet) = from_bytes::<Packet<E>>(&bytes) {
                            Self::dispatch_event(&inner, packet);
                        }
                    }
                },
            ))
        };

        dc.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
        on_msg.forget();

        let on_open = {
            let inner = inner.clone();
            Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
                Logger::info(&"DataChannel OPEN");

                let (dc, messages) = {
                    let mut inner_mut = inner.borrow_mut();
                    inner_mut.state = ConnectionState::Connected;
                    let dc = inner_mut.dc.clone();
                    let msgs: Vec<Vec<u8>> = inner_mut.message_queue.drain(..).collect();
                    (dc, msgs)
                };

                if let Some(dc) = dc {
                    for msg in messages {
                        if let Err(e) = dc.send_with_u8_array(&msg) {
                            Logger::warn(&format!("Failed to send buffered message: {:?}", e));
                        }
                    }
                }
            }))
        };
        dc.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        inner.borrow_mut().dc = Some(dc);
    }
}
