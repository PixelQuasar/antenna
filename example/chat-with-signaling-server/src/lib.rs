use antenna::{IceServerConfig, Peer, PeerID, Rtc, SignalingClient};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    pub text: String,
}

#[wasm_bindgen]
pub struct ChatApp {
    peer: Peer<Message>,
    signaling: Option<SignalingClient>,
}

#[wasm_bindgen]
impl ChatApp {
    #[wasm_bindgen(constructor)]
    pub fn new(turn_url: String) -> Result<ChatApp, JsValue> {
        console_error_panic_hook::set_once();

        let mut ice_servers = vec![IceServerConfig::new(vec![
            "stun:stun.l.google.com:19302".into(),
        ])];

        let user = "user";
        let pass = "password";

        ice_servers.push(IceServerConfig::with_credentials(
            vec![turn_url],
            user.into(),
            pass.into(),
        ));

        let mut peer = Peer::with_ice_servers(ice_servers);
        peer.subscribe(Rtc::UserMessage(Self::on_message));

        Ok(ChatApp {
            peer,
            signaling: None,
        })
    }

    #[wasm_bindgen]
    pub async fn connect(&mut self, ws_url: &str) -> Result<(), JsValue> {
        let signaling = SignalingClient::connect(ws_url)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.signaling = Some(signaling);
        Ok(())
    }

    #[wasm_bindgen(js_name = myId)]
    pub fn my_id(&self) -> String {
        self.peer.my_id().to_string()
    }

    #[wasm_bindgen]
    pub async fn join(&mut self, room_id: &str) -> Result<(), JsValue> {
        if let Some(signaling) = &mut self.signaling {
            signaling
                .join(room_id, &mut self.peer)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str(
                "Signaling client instance not found: try calling .connect(url) method first",
            ))
        }
    }

    #[wasm_bindgen(js_name = broadcast)]
    pub async fn broadcast(&mut self, text: String) -> Result<(), JsValue> {
        self.peer
            .broadcast(Message { text })
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    fn on_message(peer: PeerID, data: Message) {
        web_sys::console::log_2(
            &JsValue::from_str(&peer.as_str()),
            &JsValue::from_str(&data.text),
        );
    }

    #[wasm_bindgen(js_name = onMessage)]
    pub fn js_on_message(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_message(cb);
    }

    #[wasm_bindgen(js_name = onConnected)]
    pub fn js_on_connected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_connected(cb);
    }

    #[wasm_bindgen(js_name = onDisconnected)]
    pub fn js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_disconnected(cb);
    }

    #[wasm_bindgen(js_name = onPeerConnected)]
    pub fn js_on_peer_connected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_peer_connected(cb);
    }

    #[wasm_bindgen(js_name = onPeerDisconnected)]
    pub fn js_on_peer_disconnected(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_peer_disconnected(cb);
    }

    #[wasm_bindgen(js_name = onPeerAvailable)]
    pub fn js_on_peer_available(&mut self, cb: js_sys::Function) {
        self.peer.set_js_on_peer_available(cb);
    }

    #[wasm_bindgen(js_name = connectedPeers)]
    pub fn connected_peers(&self) -> js_sys::Array {
        self.peer
            .connected_peers()
            .into_iter()
            .map(|p| JsValue::from_str(p.as_str()))
            .collect()
    }
}
