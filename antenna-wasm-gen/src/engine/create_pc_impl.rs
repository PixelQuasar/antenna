use std::cell::RefCell;
use std::rc::Rc;

use antenna_core::{Message, SignalMessage};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::prelude::*;

use crate::AntennaEngine;
use crate::engine::EngineService;
use crate::logger::Logger;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(super) fn create_pc(
        service: &Rc<RefCell<EngineService>>,
    ) -> Result<web_sys::RtcPeerConnection, JsValue> {
        let rtc_config = web_sys::RtcConfiguration::new();
        let ice_servers_arr = js_sys::Array::new();

        let service_ref = service.borrow();
        if let Some(servers) = &service_ref.ice_servers {
            for server_config in servers {
                let rtc_ice_server = web_sys::RtcIceServer::new();

                let urls = js_sys::Array::new();
                for url in &server_config.urls {
                    urls.push(&JsValue::from_str(url));
                }
                rtc_ice_server.set_urls(&urls);

                if let Some(username) = &server_config.username {
                    rtc_ice_server.set_username(username);
                }

                if let Some(credential) = &server_config.credential {
                    rtc_ice_server.set_credential(credential);
                }

                ice_servers_arr.push(&rtc_ice_server);
            }
        }

        rtc_config.set_ice_servers(&ice_servers_arr);

        let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config)?;

        let service_for_ice = service.clone();
        let onice = Closure::wrap(Box::new(move |ev: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let msg = SignalMessage::IceCandidate {
                    candidate: candidate.candidate(),
                    sdp_mid: candidate.sdp_mid(),
                    sdp_m_line_index: candidate.sdp_m_line_index(),
                };
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Some(ws) = &service_for_ice.borrow().ws {
                        let _ = ws.send_with_str(&json);
                    }
                }
            }
        })
            as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);

        pc.set_onicecandidate(Some(onice.as_ref().unchecked_ref()));
        onice.forget();
        let pc_clone = pc.clone();
        let service_clone = service.clone();

        let oniceconnectionstatechange = Closure::wrap(Box::new(move || {
            let state = pc_clone.ice_connection_state();
            Logger::info(&format!("ICE Connection State: {:?}", state));

            // Обрабатываем другие состояния и выходим, если это не Failed
            if state != web_sys::RtcIceConnectionState::Failed {
                if state == web_sys::RtcIceConnectionState::Disconnected {
                    Logger::warn("ICE Connection Disconnected. It might recover.");
                }
                return;
            }

            Logger::error(&JsValue::from_str(
                "ICE Connection Failed! Need to restart ICE.",
            ));

            let service = service_clone.clone();
            let pc = pc_clone.clone();

            wasm_bindgen_futures::spawn_local(async move {
                Logger::info("Attempting ICE restart...");

                let options = web_sys::RtcOfferOptions::new();
                options.set_ice_restart(true);

                let offer_val = match wasm_bindgen_futures::JsFuture::from(
                    pc.create_offer_with_rtc_offer_options(&options),
                )
                .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        Logger::error(&e);
                        return;
                    }
                };

                let offer: web_sys::RtcSessionDescription = offer_val.unchecked_into();
                let offer_sdp = offer.sdp();

                let desc = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
                desc.set_sdp(&offer_sdp);

                if let Err(e) =
                    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc)).await
                {
                    Logger::error(&e);
                    return;
                }

                Logger::info("Sending ICE restart OFFER to server...");

                let msg = SignalMessage::Offer { sdp: offer_sdp };
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        Logger::error(&JsValue::from_str(&format!("JSON error: {}", e)));
                        return;
                    }
                };

                if let Some(ws) = &service.borrow().ws {
                    if let Err(e) = ws.send_with_str(&json) {
                        Logger::error(&e);
                    }
                }
            });
        }) as Box<dyn FnMut()>);

        pc.set_oniceconnectionstatechange(Some(
            oniceconnectionstatechange.as_ref().unchecked_ref(),
        ));
        oniceconnectionstatechange.forget();

        let pc_clone = pc.clone();
        let onconnectionstatechange = Closure::wrap(Box::new(move || {
            let state = pc_clone.connection_state();
            Logger::info(&format!("Peer Connection State: {:?}", state));
        }) as Box<dyn FnMut()>);

        pc.set_onconnectionstatechange(Some(onconnectionstatechange.as_ref().unchecked_ref()));
        onconnectionstatechange.forget();

        Ok(pc)
    }
}
