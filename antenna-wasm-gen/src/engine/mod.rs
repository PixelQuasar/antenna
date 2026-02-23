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
    pub room_id: String,
    pub ice_servers: Option<Vec<IceServerConfig>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IcePayload {
    candidate: String,
    sdp_mid: Option<String>,
    sdp_m_line_index: Option<u16>,
}

struct EngineService {
    state: ConnectionState,
    ws: Option<web_sys::WebSocket>,
    pc: Option<web_sys::RtcPeerConnection>,
    dc: Option<web_sys::RtcDataChannel>,
    message_queue: Vec<Vec<u8>>,
    event_handler: Option<js_sys::Function>,
    track_callback: Option<js_sys::Function>,
    ice_servers: Option<Vec<IceServerConfig>>,
}

pub struct AntennaEngine<T, E> {
    service: Rc<RefCell<EngineService>>,
    _phantom_in: std::marker::PhantomData<T>,
    _phantom_out: std::marker::PhantomData<E>,
}

impl<T, E> AntennaEngine<T, E>
where
    T: Message,
    E: Message,
{
    pub fn new(config: EngineConfig) -> Result<Self, JsValue> {
        let service = Rc::new(RefCell::new(EngineService {
            state: ConnectionState::Disconnected,
            ws: None,
            pc: None,
            dc: None,
            message_queue: Vec::new(),
            event_handler: None,
            track_callback: None,
            ice_servers: config.ice_servers.clone(),
        }));

        let engine = AntennaEngine {
            service,
            _phantom_in: std::marker::PhantomData,
            _phantom_out: std::marker::PhantomData,
        };

        engine.ws_setup(config)?;
        Ok(engine)
    }

    fn dispatch_event(service: &Rc<RefCell<EngineService>>, packet: Packet<E>) {
        if let Some(cb) = &service.borrow().event_handler
            && let Packet::User(event) = packet
            && let Ok(js_val) = serde_wasm_bindgen::to_value(&event)
        {
            let _ = cb.call1(&JsValue::NULL, &js_val);
        }
    }

    pub fn send(&self, msg: T) {
        let mut service = self.service.borrow_mut();
        let packet = Packet::User(msg);
        let bytes = to_allocvec(&packet).unwrap();
        if let Some(dc) = &service.dc
            && dc.ready_state() == web_sys::RtcDataChannelState::Open
        {
            let _ = dc.send_with_u8_array(&bytes);
            return;
        }
        service.message_queue.push(bytes);
    }

    pub fn set_event_handler(&self, event_handler: js_sys::Function) {
        self.service.borrow_mut().event_handler = Some(event_handler);
    }

    pub fn set_track_handler(&self, callback: js_sys::Function) {
        self.service.borrow_mut().track_callback = Some(callback);
    }

    pub fn add_track(
        &self,
        track: web_sys::MediaStreamTrack,
        stream: web_sys::MediaStream,
    ) -> Result<(), JsValue> {
        if let Some(pc) = &self.service.borrow().pc {
            pc.add_track_0(&track, &stream);
        }
        Ok(())
    }
}
