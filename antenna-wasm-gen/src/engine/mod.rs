use crate::logger::Logger;
use antenna_core::Message;
use antenna_core::Packet;

use antenna_core::{IceServerConfig, SignalMessage};
use postcard::{from_bytes, to_allocvec};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

mod create_pc_impl;
mod handle_signal_impl;
mod ws_setup_impl;

#[derive(Clone)]
pub struct EngineConfig {
    pub url: String,
    pub auth_token: String,
    pub ice_servers: Option<Vec<IceServerConfig>>,
}

/// Antenna client room
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InnerIce {
    candidate: String,
    sdp_mid: Option<String>,
    sdp_m_line_index: Option<u16>,
}

struct EngineInner {
    state: ConnectionState,
    ws: Option<web_sys::WebSocket>,
    pc: Option<web_sys::RtcPeerConnection>,
    dc: Option<web_sys::RtcDataChannel>,
    message_queue: Vec<Vec<u8>>,
    js_callback: Option<js_sys::Function>,
    ice_servers: Option<Vec<IceServerConfig>>,
}

pub struct AntennaEngine<T, E> {
    inner: Rc<RefCell<EngineInner>>,
    _phantom_in: std::marker::PhantomData<T>,
    _phantom_out: std::marker::PhantomData<E>,
}

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub fn new(config: EngineConfig) -> Result<Self, JsValue> {
        let inner = Rc::new(RefCell::new(EngineInner {
            state: ConnectionState::Disconnected,
            ws: None,
            pc: None,
            dc: None,
            message_queue: Vec::new(),
            js_callback: None,
            ice_servers: config.ice_servers.clone(),
        }));

        let engine = AntennaEngine {
            inner,
            _phantom_in: std::marker::PhantomData,
            _phantom_out: std::marker::PhantomData,
        };

        engine.ws_setup(config)?;
        Ok(engine)
    }

    async fn initiate_connection(inner: Rc<RefCell<EngineInner>>) {
        let pc = Self::create_pc(&inner).expect("Failed to create PC");

        let dc = pc.create_data_channel("chat");
        Self::setup_data_channel(&inner, dc);

        let offer_promise = pc.create_offer();
        let offer_val = wasm_bindgen_futures::JsFuture::from(offer_promise)
            .await
            .unwrap();
        let offer_sdp = js_sys::Reflect::get(&offer_val, &"sdp".into())
            .unwrap()
            .as_string()
            .unwrap();

        let desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
        desc.set_sdp(&offer_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc))
            .await
            .unwrap();

        Logger::info(&"Sending OFFER to server...");
        let msg = SignalMessage::Offer { sdp: offer_sdp };
        let json = serde_json::to_string(&msg).unwrap();

        inner.borrow_mut().pc = Some(pc);
        if let Some(ws) = &inner.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }
    }

    async fn handle_remote_offer(inner: Rc<RefCell<EngineInner>>, remote_sdp: String) {
        let pc = Self::create_pc(&inner).expect("Failed to create PC");

        let inner_dc = inner.clone();
        let ondatachannel_callback =
            Closure::wrap(Box::new(move |ev: web_sys::RtcDataChannelEvent| {
                let dc = ev.channel();
                Logger::info(&format!("Received DataChannel: {}", dc.label()));
                Self::setup_data_channel(&inner_dc, dc);
            })
                as Box<dyn FnMut(web_sys::RtcDataChannelEvent)>);
        pc.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
        ondatachannel_callback.forget();

        let desc_init = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
        desc_init.set_sdp(&remote_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&desc_init))
            .await
            .unwrap();

        let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer())
            .await
            .unwrap();
        let answer_sdp = js_sys::Reflect::get(&answer, &"sdp".into())
            .unwrap()
            .as_string()
            .unwrap();

        // Set Local
        let answer_init = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
        answer_init.set_sdp(&answer_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init))
            .await
            .unwrap();

        Logger::info(&"Sending ANSWER to server...");
        let msg = SignalMessage::Answer { sdp: answer_sdp };
        let json = serde_json::to_string(&msg).unwrap();

        inner.borrow_mut().pc = Some(pc);
        if let Some(ws) = &inner.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }
    }

    fn setup_data_channel(inner: &Rc<RefCell<EngineInner>>, dc: web_sys::RtcDataChannel) {
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
                inner.borrow_mut().state = ConnectionState::Connected;

                if let Some(dc) = &inner.borrow().dc {
                    for msg in inner.borrow_mut().message_queue.drain(..) {
                        let _ = dc.send_with_u8_array(&msg);
                    }
                }
            }))
        };
        dc.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        inner.borrow_mut().dc = Some(dc);
    }

    fn dispatch_event(inner: &Rc<RefCell<EngineInner>>, packet: Packet<E>) {
        if let Some(cb) = &inner.borrow().js_callback {
            match packet {
                Packet::User(event) => {
                    if let Ok(js_val) = serde_wasm_bindgen::to_value(&event) {
                        let _ = cb.call1(&JsValue::NULL, &js_val);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn send(&self, msg: T) {
        let mut inner = self.inner.borrow_mut();
        let packet = Packet::User(msg);
        let bytes = to_allocvec(&packet).unwrap();
        if let Some(dc) = &inner.dc {
            if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                let _ = dc.send_with_u8_array(&bytes);
                return;
            }
        }
        inner.message_queue.push(bytes);
    }

    pub fn set_event_handler(&self, callback: js_sys::Function) {
        self.inner.borrow_mut().js_callback = Some(callback);
    }
}
