use crate::logger::Logger;
use antenna_core::utils::DEFAULT_STUN_ADDR;
use antenna_core::model::Packet;
use antenna_core::model::SignalMessage;
use antenna_core::traits::message::AntennaMessage;
use postcard::{from_bytes, to_allocvec};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Client-side input for setting up Antenna Engine
#[derive(Clone)]
pub struct EngineConfig {
    /// Signaling server URL
    pub url: String,
    /// Auth token
    pub auth_token: String,
}

/// Antenna client room
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    /// Disconnected from peer
    Disconnected,
    /// Initiation of connection in progress (offering / sending own ICE candidate)
    Connecting,
    /// Connected to peer
    Connected,
}

struct EngineInner {
    state: ConnectionState,
    ws: Option<web_sys::WebSocket>,
    pc: Option<web_sys::RtcPeerConnection>,
    dc: Option<web_sys::RtcDataChannel>,
    message_queue: Vec<Vec<u8>>,
    js_callback: Option<js_sys::Function>,
}

pub struct AntennaEngine<T, E> {
    inner: Rc<RefCell<EngineInner>>,
    _phantom_in: std::marker::PhantomData<T>,
    _phantom_out: std::marker::PhantomData<E>,
}

impl<T, E> AntennaEngine<T, E>
where
    T: AntennaMessage,           // Input
    E: AntennaMessage + 'static, // Output
{
    pub fn new(config: EngineConfig) -> Result<Self, JsValue> {
        let inner = Rc::new(RefCell::new(EngineInner {
            state: ConnectionState::Disconnected,
            ws: None,
            pc: None,
            dc: None,
            message_queue: Vec::new(),
            js_callback: None,
        }));

        let engine = AntennaEngine {
            inner,
            _phantom_in: std::marker::PhantomData,
            _phantom_out: std::marker::PhantomData,
        };

        engine.ws_setup(config)?;

        Ok(engine)
    }

    /// Setting up WS client and initiating basic connection
    fn ws_setup(&self, config: EngineConfig) -> Result<(), JsValue> {
        let ws = web_sys::WebSocket::new(&config.url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let onopen_callback = {
            let inner = self.inner.clone();
            let token = config.auth_token.clone();
            Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
                web_sys::console::log_1(&"WS Open".into());

                let join_msg = SignalMessage::Join {
                    room: "DEFAULT".to_string(), // TODO add dynamic room param
                    token: Some(token.clone()),
                };

                let json = serde_json::to_string(&join_msg).unwrap();
                let wss = inner.borrow().ws.clone().unwrap();
                wss.send_with_str(&json).unwrap();

                inner.borrow_mut().state = ConnectionState::Connecting;
            }))
        };
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let onmessage_callback = {
            let inner = self.inner.clone();
            Closure::<dyn FnMut(web_sys::MessageEvent)>::wrap(Box::new(
                move |e: web_sys::MessageEvent| {
                    if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                        let text = text.into();
                        Self::handle_signal(&inner, text);
                    }
                },
            ))
        };
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        self.inner.borrow_mut().ws = Some(ws);

        Ok(())
    }

    /// handle basic signaling messages - getting offer from other peer and getting ice candidate
    fn handle_signal(inner: &Rc<RefCell<EngineInner>>, text: String) {
        let msg: SignalMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                return web_sys::console::warn_1(
                    &Logger::warn_msg(&format!("Signal parse error: {}", e)).into(),
                );
            }
        };

        match msg {
            SignalMessage::Offer { sdp } => {
                web_sys::console::log_1(&"Got Offer".into());
                let inner = inner.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    Self::setup_peer_connection_and_answer(inner.clone(), sdp).await;
                });
            }
            SignalMessage::IceCandidate {
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => {
                // Добавляем кандидата в PC
                if let Some(pc) = &inner.borrow().pc {
                    let mut init = web_sys::RtcIceCandidateInit::new(&candidate);
                    init.sdp_mid(sdp_mid.as_deref());
                    init.sdp_m_line_index(sdp_m_line_index);
                    let candidate_obj = web_sys::RtcIceCandidate::new(&init).unwrap();
                    let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate_obj));
                }
            }
            _ => {}
        }
    }

    /// Initiating PeerConnection, setting up offer, sending answer
    async fn setup_peer_connection_and_answer(inner: Rc<RefCell<EngineInner>>, remote_sdp: String) {
        let rtc_config = web_sys::RtcConfiguration::new();
        let ice_server = web_sys::RtcIceServer::new();
        ice_server.set_urls(&JsValue::from_str(DEFAULT_STUN_ADDR));
        let ice_servers_arr = js_sys::Array::new();
        ice_servers_arr.push(&ice_server);
        rtc_config.set_ice_servers(&ice_servers_arr);
        let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config).unwrap();

        let ondatachannel_callback = {
            let inner = inner.clone();
            Closure::<dyn FnMut(web_sys::RtcDataChannelEvent)>::wrap(Box::new(
                move |ev: web_sys::RtcDataChannelEvent| {
                    let dc = ev.channel();
                    web_sys::console::log_1(
                        &format!("DataChannel received: {}", dc.label()).into(),
                    );
                    Self::setup_data_channel(&inner, dc);
                },
            ))
        };
        pc.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
        ondatachannel_callback.forget();

        let onicecandidate_callback = {
            let inner = inner.clone();
            Closure::<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>::wrap(Box::new(
                move |ev: web_sys::RtcPeerConnectionIceEvent| {
                    if let Some(candidate) = ev.candidate() {
                        let msg = SignalMessage::IceCandidate {
                            candidate: candidate.candidate(),
                            sdp_mid: candidate.sdp_mid(),
                            sdp_m_line_index: candidate.sdp_m_line_index(),
                        };
                        let json = serde_json::to_string(&msg).unwrap();
                        if let Some(ws) = &inner.borrow().ws {
                            ws.send_with_str(&json).unwrap();
                        }
                    }
                },
            ))
        };
        pc.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();

        // --- Logic ---

        let mut desc_init = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
        desc_init.sdp(&remote_sdp);

        // setRemoteDescription
        let _ = wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&desc_init)).await;

        // createAnswer
        let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer())
            .await
            .unwrap();
        let answer_sdp = js_sys::Reflect::get(&answer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut answer_init = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
        answer_init.set_sdp(&answer_sdp);

        // setLocalDescription
        let _ = wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init)).await;

        // Отправка Answer по WS
        let msg = SignalMessage::Answer { sdp: answer_sdp };
        let json = serde_json::to_string(&msg).unwrap();
        if let Some(ws) = &inner.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }

        inner.borrow_mut().pc = Some(pc);
    }

    /// Настройка Data Channel
    fn setup_data_channel(inner: &Rc<RefCell<EngineInner>>, dc: web_sys::RtcDataChannel) {
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

        // On Message (Входящие данные)
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

        // On Open (Канал готов!)
        let on_open = {
            let inner = inner.clone();
            Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
                web_sys::console::log_1(&"DataChannel OPEN".into());
                inner.borrow_mut().state = ConnectionState::Connected;

                // TODO: Отправить все сообщения из очереди
            }))
        };
        dc.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        inner.borrow_mut().dc = Some(dc);
    }

    /// Отправка события в JS
    fn dispatch_event(inner: &Rc<RefCell<EngineInner>>, packet: Packet<E>) {
        if let Some(cb) = &inner.borrow().js_callback {
            match packet {
                Packet::User(event) => {
                    if let Ok(js_val) = serde_wasm_bindgen::to_value(&event) {
                        let _ = cb.call1(&JsValue::NULL, &js_val);
                    }
                }
                Packet::System(sys) => {
                    // Обработка пингов и прочего
                }
                _ => {}
            }
        }
    }

    /// Публичный метод: Отправка данных
    pub fn send(&self, msg: T) {
        let mut inner = self.inner.borrow_mut();

        // Упаковка Packet::User(msg)
        let packet = Packet::User(msg);
        let bytes = to_allocvec(&packet).unwrap();

        if let Some(dc) = &inner.dc {
            if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                let _ = dc.send_with_u8_array(&bytes);
                return;
            }
        }

        // Если не подключено - в очередь
        inner.message_queue.push(bytes);
    }

    pub fn set_event_handler(&self, callback: js_sys::Function) {
        self.inner.borrow_mut().js_callback = Some(callback);
    }
}
