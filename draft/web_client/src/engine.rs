use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

use crate::MeshNode;
use antenna_protocol::IceServer;

pub struct EngineConfig {
    pub url: String,
    pub room_id: String,
    pub ice_servers: Option<Vec<IceServer>>,
}

pub struct AntennaEngine<M> {
    node: MeshNode,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> AntennaEngine<M>
where
    M: Serialize + DeserializeOwned + 'static,
{
    pub fn new(config: EngineConfig) -> Result<Self, JsValue> {
        // For now, we just create a master node. In a real implementation,
        // this would connect to the signaling server using `config.url` and `config.room_id`.
        let ice_servers = config
            .ice_servers
            .unwrap_or_else(|| vec![IceServer::default_stun()]);
        let node = MeshNode::new_master(ice_servers);

        Ok(Self {
            node,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn send(&self, msg: M) {
        let _ = self.node.broadcast_json(&msg);
    }

    pub fn set_event_handler(&self, cb: js_sys::Function) {
        self.node.on_json_message(move |_peer_id, msg: M| {
            if let Ok(js_val) = serde_wasm_bindgen::to_value(&msg) {
                let _ = cb.call1(&JsValue::NULL, &js_val);
            }
        });
    }
}
