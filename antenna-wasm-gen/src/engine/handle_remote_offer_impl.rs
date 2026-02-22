use crate::AntennaEngine;
use crate::engine::EngineService;
use crate::logger::Logger;
use antenna_core::Message;

use antenna_core::SignalMessage;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

impl<T, E> AntennaEngine<T, E>
where
    T: Message + Clone + 'static,
    E: Message + 'static,
{
    pub(super) async fn handle_remote_offer(
        service: Rc<RefCell<EngineService>>,
        remote_sdp: String,
    ) {
        let pc = Self::create_pc(&service).expect("Failed to create PC");

        let service_clone = service.clone();
        let ondatachannel_callback =
            Closure::wrap(Box::new(move |ev: web_sys::RtcDataChannelEvent| {
                let dc = ev.channel();
                Logger::info(&format!("Received DataChannel: {}", dc.label()));
                Self::setup_data_channel(&service_clone, dc);
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

        service.borrow_mut().pc = Some(pc);
        if let Some(ws) = &service.borrow().ws {
            ws.send_with_str(&json).unwrap();
        }
    }
}
