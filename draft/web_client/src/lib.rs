//! # antenna-web-impl
//!
//! WASM/browser IO layer for the antenna serverless P2P WebRTC mesh.
//!
//! Provides [`MeshNode`] with a **context/reducer** pattern:
//! the consumer defines a context type and a reducer; the library
//! calls the reducer on every mesh event and fires `on_change`.

pub mod driver;
pub mod engine;
pub mod webrtc;

use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

use antenna_protocol::{Answer, IceServer, Invite, Mesh, PeerId};

pub use antenna_protocol;
pub use driver::Shared;
pub use engine::{AntennaEngine, EngineConfig};

// ---------------------------------------------------------------------------
// MeshEvent
// ---------------------------------------------------------------------------

/// Events delivered to the consumer's reducer.
pub enum MeshEvent<M> {
    /// A typed message was received from a peer.
    MessageReceived { from: PeerId, message: M },
    /// A peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
}

// ---------------------------------------------------------------------------
// MeshNode
// ---------------------------------------------------------------------------

pub struct MeshNode {
    inner: Shared,
}

impl MeshNode {
    pub fn new_master(ice_servers: Vec<IceServer>) -> Self {
        Self {
            inner: driver::new_shared(Mesh::new_master(ice_servers)),
        }
    }

    pub fn new_joiner() -> Self {
        Self {
            inner: driver::new_shared(Mesh::new_joiner()),
        }
    }

    // -- Context/reducer ----------------------------------------------------

    /// Set up the context/reducer pattern.
    ///
    /// Returns an `Rc<RefCell<C>>` handle so the consumer can read/write
    /// the context (e.g. to add local messages, toggle typing state).
    ///
    /// The library calls `reducer` on every mesh event, then `on_change`
    /// with the updated context.
    pub fn on_json_message<M>(&self, cb: impl Fn(PeerId, M) + 'static)
    where
        M: DeserializeOwned + 'static,
    {
        self.inner.borrow_mut().on_message = Some(Box::new(move |from, data| {
            if let Ok(message) = serde_json::from_slice::<M>(data) {
                cb(from.clone(), message);
            }
        }));
    }

    pub fn on_peer_connected(&self, cb: impl Fn(PeerId) + 'static) {
        self.inner.borrow_mut().on_peer_connected = Some(Box::new(move |peer| cb(peer.clone())));
    }

    pub fn on_peer_disconnected(&self, cb: impl Fn(PeerId) + 'static) {
        self.inner.borrow_mut().on_peer_disconnected = Some(Box::new(move |peer| cb(peer.clone())));
    }

    // -- Bootstrap ----------------------------------------------------------

    pub fn generate_invite(&self) -> Result<(), JsValue> {
        let outputs = {
            let mut inner = self.inner.borrow_mut();
            inner
                .mesh
                .create_invite()
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };
        for output in outputs {
            driver::process_output(&self.inner, output);
        }
        Ok(())
    }

    pub fn accept_answer(&self, answer_json: &str) -> Result<(), JsValue> {
        let answer: Answer = serde_json::from_str(answer_json)
            .map_err(|e| JsValue::from_str(&format!("invalid answer: {e}")))?;
        driver::feed_input(&self.inner, antenna_protocol::Input::AnswerReceived(answer));
        Ok(())
    }

    pub fn accept_invite(&self, invite_json: &str) -> Result<(), JsValue> {
        let invite: Invite = serde_json::from_str(invite_json)
            .map_err(|e| JsValue::from_str(&format!("invalid invite: {e}")))?;
        driver::feed_input(&self.inner, antenna_protocol::Input::InviteReceived(invite));
        Ok(())
    }

    pub fn on_invite_ready(&self, cb: impl Fn(String) + 'static) {
        self.inner.borrow_mut().on_invite_ready = Some(Box::new(cb));
    }

    pub fn on_answer_ready(&self, cb: impl Fn(String) + 'static) {
        self.inner.borrow_mut().on_answer_ready = Some(Box::new(cb));
    }

    // -- Peer tracking ------------------------------------------------------

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.inner
            .borrow()
            .connected_peers
            .iter()
            .cloned()
            .collect()
    }

    // -- Messaging ----------------------------------------------------------

    pub fn broadcast(&self, data: &[u8]) -> Result<(), JsValue> {
        let inner = self.inner.borrow();
        if let Some(ref dc) = inner.bootstrap_dc {
            if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                dc.send_with_u8_array(data)?;
            }
        }
        for res in inner.peers.values() {
            if let Some(dc) = &res.dc {
                if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                    dc.send_with_u8_array(data)?;
                }
            }
        }
        Ok(())
    }

    pub fn broadcast_json<T: Serialize>(&self, msg: &T) -> Result<(), JsValue> {
        let bytes =
            serde_json::to_vec(msg).map_err(|e| JsValue::from_str(&format!("serialize: {e}")))?;
        self.broadcast(&bytes)
    }

    pub fn send(&self, to: &PeerId, data: &[u8]) -> Result<(), JsValue> {
        let inner = self.inner.borrow();
        if let Some(res) = inner.peers.get(to) {
            let dc = res
                .dc
                .as_ref()
                .ok_or_else(|| JsValue::from_str(&format!("no DC for {to}")))?;
            dc.send_with_u8_array(data)?;
            return Ok(());
        }
        if let Some(ref dc) = inner.bootstrap_dc {
            if dc.ready_state() == web_sys::RtcDataChannelState::Open {
                dc.send_with_u8_array(data)?;
                return Ok(());
            }
        }
        Err(JsValue::from_str(&format!("no connection to {to}")))
    }

    pub fn send_json<T: Serialize>(&self, to: &PeerId, msg: &T) -> Result<(), JsValue> {
        let bytes =
            serde_json::to_vec(msg).map_err(|e| JsValue::from_str(&format!("serialize: {e}")))?;
        self.send(to, &bytes)
    }
}

pub fn log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}
