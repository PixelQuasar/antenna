use crate::AntennaEngine;
use crate::engine::EngineService;
use crate::logger::Logger;
use antenna_core::Message;

use antenna_core::SignalMessage;
use std::cell::RefCell;
use std::rc::Rc;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(super) async fn init_connection(service: Rc<RefCell<EngineService>>) {
        let pc = Self::create_pc(&service).expect("Failed to create PC");

        let dc = pc.create_data_channel("chat");
        Self::setup_data_channel(&service, dc);

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

        service.borrow_mut().pc = Some(pc);
        if let Some(ws) = &service.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }

        let service_clone = service.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 5000)
                        .unwrap();
                });
                wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();

                let service = service_clone.borrow();
                if let Some(dc) = &service.dc {
                    if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                        let ping: antenna_core::Packet<T> =
                            antenna_core::Packet::System(antenna_core::SystemMessage::Ping {
                                timestamp: js_sys::Date::now() as u64,
                            });
                        if let Ok(bytes) = postcard::to_allocvec(&ping) {
                            let _ = dc.send_with_u8_array(&bytes);
                        }
                    } else if dc.ready_state() == web_sys::RtcDataChannelState::Closed {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
    }
}
