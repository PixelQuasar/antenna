use crate::utils::to_js_object;
use antenna_client_shared::{MessageCallback, NoArgCallback, PeerCallback};
use antenna_protocol::UserMsgPayload;
use anyhow::anyhow;
use send_wrapper::SendWrapper;
use wasm_bindgen::prelude::*;

/// Wrap a JS function (zero arguments) into a [`NoArgCallback`] for
/// `Peer::subscribe(Event::Connected(...))` and similar.
pub fn js_no_arg(f: js_sys::Function) -> NoArgCallback {
    let f = SendWrapper::new(f);
    NoArgCallback::from_fn(move || {
        f.call0(&JsValue::NULL)
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to call JS callback: {:#?}", e))
    })
}

/// Wrap a JS function `(peerId: string)` into a [`PeerCallback`] for
/// `Event::PeerConnected` and similar peer-scoped events.
pub fn js_peer(f: js_sys::Function) -> PeerCallback {
    let f = SendWrapper::new(f);
    PeerCallback::from_fn(move |peer| {
        let peer = js_sys::JsString::from(peer.to_string());
        f.call1(&JsValue::NULL, &peer)
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to call JS peer callback: {:#?}", e))
    })
}

/// Wrap a JS function `(peerId: string, msg: object)` into a
/// [`MessageCallback`] for `Event::UserMessage`. `msg` is serialized to a
/// plain JS object via `serde-wasm-bindgen`.
pub fn js_message<Msg: UserMsgPayload + 'static>(f: js_sys::Function) -> MessageCallback<Msg> {
    let f = SendWrapper::new(f);
    MessageCallback::from_fn(move |peer, data| {
        let msg_obj = to_js_object(data)
            .map_err(|e| anyhow!("Failed to serialize message payload: {:#?}", e))?;
        let peer = js_sys::JsString::from(peer.to_string());
        f.call2(&JsValue::NULL, &peer, &msg_obj)
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to call JS message callback: {:#?}", e))
    })
}
