use std::cell::RefCell;
use std::rc::Rc;

use antenna_core::{Message, SignalMessage};

use crate::AntennaEngine;
use crate::engine::{EngineService, IcePayload};
use crate::logger::Logger;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(super) fn handle_signal(service: &Rc<RefCell<EngineService>>, text: String) {
        let msg: SignalMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let err_text = format!("JSON Error: {}. Text: {}", e, text);
                Logger::warn(&err_text);
                return;
            }
        };

        let service = service.clone();

        match msg {
            SignalMessage::IceConfig { ice_servers } => {
                Logger::info(&format!(
                    "Received ICE Config: {} servers",
                    ice_servers.len()
                ));
                service.borrow_mut().ice_servers = Some(ice_servers);
            }

            SignalMessage::Welcome { .. } => {
                Logger::info(&"Received Welcome. Initiating connection...");
                wasm_bindgen_futures::spawn_local(async move {
                    Self::init_connection(service).await;
                });
            }

            SignalMessage::Offer { sdp } => {
                Logger::info(&"Received Offer from Server");
                wasm_bindgen_futures::spawn_local(async move {
                    Self::handle_remote_offer(service, sdp).await;
                });
            }

            SignalMessage::Answer { sdp } => {
                Logger::info(&"Received Answer from Server");
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(pc) = service.borrow().pc.clone() {
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

            SignalMessage::IceCandidate {
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => {
                if let Some(pc) = service.borrow().pc.clone() {
                    let (real_candidate, real_mid, real_idx) = if candidate.trim().starts_with('{')
                    {
                        match serde_json::from_str::<IcePayload>(&candidate) {
                            Ok(payload) => {
                                (payload.candidate, payload.sdp_mid, payload.sdp_m_line_index)
                            }
                            Err(e) => {
                                Logger::warn(&format!("Failed to parse ICE payload json: {}", e));
                                (candidate, sdp_mid, sdp_m_line_index)
                            }
                        }
                    } else {
                        (candidate, sdp_mid, sdp_m_line_index)
                    };

                    let init = web_sys::RtcIceCandidateInit::new(&real_candidate);

                    if let Some(mid) = real_mid {
                        init.set_sdp_mid(Some(&mid));
                    }
                    if let Some(idx) = real_idx {
                        init.set_sdp_m_line_index(Some(idx));
                    }

                    Logger::info(&format!("Adding ICE: {}", real_candidate));

                    let promise = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&init));

                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                            Logger::warn(&format!("Error adding ICE: {:?}", e));
                        }
                    });
                }
            }

            _ => {}
        }
    }
}
