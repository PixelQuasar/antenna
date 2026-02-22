use crate::logger::Logger;
use antenna_core::Message;
use antenna_core::Packet;
use antenna_core::SignalMessage;
use antenna_core::utils::DEFAULT_STUN_ADDR;
use postcard::{from_bytes, to_allocvec};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
pub struct EngineConfig {
    pub url: String,
    pub auth_token: String,
}

/// Antenna client room
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(serde::Deserialize)]
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
        }));

        let engine = AntennaEngine {
            inner,
            _phantom_in: std::marker::PhantomData,
            _phantom_out: std::marker::PhantomData,
        };

        engine.ws_setup(config)?;
        Ok(engine)
    }

    fn ws_setup(&self, config: EngineConfig) -> Result<(), JsValue> {
        let ws = web_sys::WebSocket::new(&config.url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // ON OPEN
        let onopen_callback = {
            let inner = self.inner.clone();
            let token = config.auth_token.clone();
            Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
                Logger::info(&"WS Open");

                let join_msg = SignalMessage::Join {
                    room: "DEFAULT".to_string(),
                    token: Some(token.clone()),
                };

                let json = serde_json::to_string(&join_msg).unwrap();
                if let Some(ws) = &inner.borrow().ws {
                    ws.send_with_str(&json).unwrap();
                }
                inner.borrow_mut().state = ConnectionState::Connecting;
            }))
        };
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // ON MESSAGE
        let onmessage_callback = {
            let inner = self.inner.clone();
            Closure::<dyn FnMut(web_sys::MessageEvent)>::wrap(Box::new(
                move |e: web_sys::MessageEvent| {
                    if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                        let text: String = text.into();
                        // Лог входящего сырого сообщения
                        Logger::info(&format!("WS IN: {}", text));
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

    // ------------------------------------------------------------------------
    // CORE SIGNALING LOGIC
    // ------------------------------------------------------------------------

    fn handle_signal(inner_rc: &Rc<RefCell<EngineInner>>, text: String) {
        let msg: SignalMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let err_text = format!("JSON Error: {}. Text: {}", e, text);
                Logger::warn(&err_text);
                return;
            }
        };

        let inner = inner_rc.clone();

        match msg {
            // [ВАЖНО] 1. Сервер сказал "Привет" -> Мы начинаем WebRTC
            SignalMessage::Welcome { .. } => {
                Logger::info(&"Received Welcome. Initiating connection...");
                wasm_bindgen_futures::spawn_local(async move {
                    Self::initiate_connection(inner).await;
                });
            }

            // [ВАЖНО] 2. Сервер прислал Offer (если кто-то другой вошел)
            SignalMessage::Offer { sdp } => {
                Logger::info(&"Received Offer from Server");
                wasm_bindgen_futures::spawn_local(async move {
                    Self::handle_remote_offer(inner, sdp).await;
                });
            }

            // [ВАЖНО] 3. Сервер ответил на наш Offer
            SignalMessage::Answer { sdp } => {
                Logger::info(&"Received Answer from Server");
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(pc) = inner.borrow().pc.clone() {
                        let desc =
                            web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
                        desc.set_sdp(&sdp);
                        if let Err(e) =
                            wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&desc))
                                .await
                        {
                            Logger::error(&e);
                        } else {
                            Logger::info(&"Remote description set (Answer)");
                        }
                    }
                });
            }

            // 4. ICE Candidate
            SignalMessage::IceCandidate {
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => {
                if let Some(pc) = inner.borrow().pc.clone() {
                    // ЛОГИКА ИСПРАВЛЕНИЯ:
                    // Проверяем, пришел ли JSON внутри строки candidate
                    let (real_candidate, real_mid, real_idx) = if candidate.trim().starts_with('{')
                    {
                        // Пытаемся распарсить вложенный JSON от сервера
                        match serde_json::from_str::<InnerIce>(&candidate) {
                            Ok(inner) => (inner.candidate, inner.sdp_mid, inner.sdp_m_line_index),
                            Err(e) => {
                                Logger::warn(&format!("Failed to parse inner ICE json: {}", e));
                                (candidate, sdp_mid, sdp_m_line_index)
                            }
                        }
                    } else {
                        // Если пришла нормальная строка, используем как есть
                        (candidate, sdp_mid, sdp_m_line_index)
                    };

                    // Теперь создаем объект для браузера с ПРАВИЛЬНЫМИ данными
                    let init = web_sys::RtcIceCandidateInit::new(&real_candidate);

                    if let Some(mid) = real_mid {
                        init.set_sdp_mid(Some(&mid));
                    }
                    if let Some(idx) = real_idx {
                        init.set_sdp_m_line_index(Some(idx));
                    }

                    // Логируем, что добавляем
                    Logger::info(&format!("Adding ICE: {}", real_candidate));

                    let promise = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&init));

                    // Необязательно, но полезно отловить ошибку добавления
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                            Logger::warn(&format!("Error adding ICE: {:?}", e));
                        }
                    });
                }
            }
            // Игнорируем Join, так как это сообщение от клиента к серверу
            _ => {}
        }
    }

    // ------------------------------------------------------------------------
    // WEBRTC LOGIC
    // ------------------------------------------------------------------------

    // Вспомогательная функция: создание PC и ICE
    fn create_pc(inner: &Rc<RefCell<EngineInner>>) -> Result<web_sys::RtcPeerConnection, JsValue> {
        let rtc_config = web_sys::RtcConfiguration::new();
        let ice_server = web_sys::RtcIceServer::new();
        ice_server.set_urls(&JsValue::from_str(DEFAULT_STUN_ADDR));
        let ice_servers_arr = js_sys::Array::new();
        ice_servers_arr.push(&ice_server);
        rtc_config.set_ice_servers(&ice_servers_arr);

        let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config)?;

        // ICE HANDLER
        let inner_clone = inner.clone();
        let onice = Closure::wrap(Box::new(move |ev: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let msg = SignalMessage::IceCandidate {
                    candidate: candidate.candidate(),
                    sdp_mid: candidate.sdp_mid(),
                    sdp_m_line_index: candidate.sdp_m_line_index(),
                };
                // Отправляем ICE сразу
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Some(ws) = &inner_clone.borrow().ws {
                        let _ = ws.send_with_str(&json);
                    }
                }
            }
        })
            as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);

        pc.set_onicecandidate(Some(onice.as_ref().unchecked_ref()));
        onice.forget();

        Ok(pc)
    }

    /// [Сценарий 1] Мы инициаторы (после Welcome)
    async fn initiate_connection(inner: Rc<RefCell<EngineInner>>) {
        let pc = Self::create_pc(&inner).expect("Failed to create PC");

        // 1. Инициатор создает DataChannel
        let dc = pc.create_data_channel("chat");
        Self::setup_data_channel(&inner, dc);

        // 2. Создаем Offer
        let offer_promise = pc.create_offer();
        let offer_val = wasm_bindgen_futures::JsFuture::from(offer_promise)
            .await
            .unwrap();
        let offer_sdp = js_sys::Reflect::get(&offer_val, &"sdp".into())
            .unwrap()
            .as_string()
            .unwrap();

        // 3. Set Local
        let desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
        desc.set_sdp(&offer_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc))
            .await
            .unwrap();

        // 4. Send
        Logger::info(&"Sending OFFER to server...");
        let msg = SignalMessage::Offer { sdp: offer_sdp };
        let json = serde_json::to_string(&msg).unwrap();

        inner.borrow_mut().pc = Some(pc);
        if let Some(ws) = &inner.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }
    }

    /// [Сценарий 2] Мы приемники (Сервер прислал Offer)
    async fn handle_remote_offer(inner: Rc<RefCell<EngineInner>>, remote_sdp: String) {
        let pc = Self::create_pc(&inner).expect("Failed to create PC");

        // Приемник ждет DataChannel (ondatachannel)
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

        // Set Remote
        let desc_init = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
        desc_init.set_sdp(&remote_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&desc_init))
            .await
            .unwrap();

        // Create Answer
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

        // Send Answer
        Logger::info(&"Sending ANSWER to server...");
        let msg = SignalMessage::Answer { sdp: answer_sdp };
        let json = serde_json::to_string(&msg).unwrap();

        inner.borrow_mut().pc = Some(pc);
        if let Some(ws) = &inner.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }
    }

    // ------------------------------------------------------------------------
    // DATA CHANNEL & EVENTS
    // ------------------------------------------------------------------------

    fn setup_data_channel(inner: &Rc<RefCell<EngineInner>>, dc: web_sys::RtcDataChannel) {
        // ... (Этот код у вас был верным, оставляем как есть) ...
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

    // ... dispatch_event, send, set_event_handler без изменений ...
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
