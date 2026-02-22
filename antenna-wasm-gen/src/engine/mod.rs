use antenna_core::Message;
use antenna_core::Packet;

use antenna_core::IceServerConfig;
use postcard::to_allocvec;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

mod create_pc_impl;
mod handle_remote_offer_impl;
mod handle_signal_impl;
mod init_connection_impl;
mod setup_data_channel_impl;
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
