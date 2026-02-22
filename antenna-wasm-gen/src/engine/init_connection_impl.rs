use crate::AntennaEngine;
use crate::engine::EngineInner;
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
    pub(super) async fn init_connection(inner: Rc<RefCell<EngineInner>>) {
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
}
