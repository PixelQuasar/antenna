use std::cell::RefCell;
use std::rc::Rc;

use antenna_core::utils::{
    DEFAULT_STUN_ADDR, DEFAULT_STUN_ADDR_2, DEFAULT_STUN_ADDR_3, DEFAULT_STUN_ADDR_4,
};
use antenna_core::{Message, SignalMessage};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::prelude::*;

use crate::AntennaEngine;
use crate::engine::EngineInner;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(crate) fn create_pc(
        inner: &Rc<RefCell<EngineInner>>,
    ) -> Result<web_sys::RtcPeerConnection, JsValue> {
        let rtc_config = web_sys::RtcConfiguration::new();
        let ice_servers_arr = js_sys::Array::new();

        let inner_ref = inner.borrow();
        if let Some(servers) = &inner_ref.ice_servers {
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
        } else {
            let stun_urls = js_sys::Array::new();
            stun_urls.push(&JsValue::from_str(DEFAULT_STUN_ADDR));
            stun_urls.push(&JsValue::from_str(DEFAULT_STUN_ADDR_2));
            stun_urls.push(&JsValue::from_str(DEFAULT_STUN_ADDR_3));
            stun_urls.push(&JsValue::from_str(DEFAULT_STUN_ADDR_4));

            let stun_server = web_sys::RtcIceServer::new();
            stun_server.set_urls(&stun_urls);
            ice_servers_arr.push(&stun_server);
        }

        rtc_config.set_ice_servers(&ice_servers_arr);

        let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config)?;

        let inner_clone = inner.clone();
        let onice = Closure::wrap(Box::new(move |ev: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let msg = SignalMessage::IceCandidate {
                    candidate: candidate.candidate(),
                    sdp_mid: candidate.sdp_mid(),
                    sdp_m_line_index: candidate.sdp_m_line_index(),
                };
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
}
