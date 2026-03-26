use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use antenna_protocol::{Input, Mesh, Output, PeerId, RelayMessage};

use crate::webrtc::PeerResources;
use crate::{log, webrtc};

// ---------------------------------------------------------------------------
// Callback types
// ---------------------------------------------------------------------------

pub type MessageCallback = Box<dyn Fn(&PeerId, &[u8])>;
pub type PeerCallback = Box<dyn Fn(&PeerId)>;
pub type InviteReadyCallback = Box<dyn Fn(String)>;
pub type AnswerReadyCallback = Box<dyn Fn(String)>;

// ---------------------------------------------------------------------------
// Shared inner state
// ---------------------------------------------------------------------------

pub struct Inner {
    pub mesh: Mesh,
    pub bootstrap_pc: Option<web_sys::RtcPeerConnection>,
    pub bootstrap_dc: Option<web_sys::RtcDataChannel>,
    pub peers: HashMap<PeerId, PeerResources>,
    pub connected_peers: HashSet<PeerId>,

    pub on_message: Option<MessageCallback>,
    pub on_peer_connected: Option<PeerCallback>,
    pub on_peer_disconnected: Option<PeerCallback>,
    pub on_invite_ready: Option<InviteReadyCallback>,
    pub on_answer_ready: Option<AnswerReadyCallback>,
}

pub type Shared = Rc<RefCell<Inner>>;

pub fn new_shared(mesh: Mesh) -> Shared {
    Rc::new(RefCell::new(Inner {
        mesh,
        bootstrap_pc: None,
        bootstrap_dc: None,
        peers: HashMap::new(),
        connected_peers: HashSet::new(),
        on_message: None,
        on_peer_connected: None,
        on_peer_disconnected: None,
        on_invite_ready: None,
        on_answer_ready: None,
    }))
}

// ---------------------------------------------------------------------------
// Input → state machine → outputs → execute
// ---------------------------------------------------------------------------

pub fn feed_input(shared: &Shared, input: Input) {
    let outputs = {
        let mut inner = shared.borrow_mut();
        match inner.mesh.handle(input) {
            Ok(outputs) => outputs,
            Err(e) => {
                log(&format!("[antenna] state machine error: {e}"));
                return;
            }
        }
    };
    for output in outputs {
        process_output(shared, output);
    }
}

pub fn process_output(shared: &Shared, output: Output) {
    match output {
        Output::CreateBootstrapPeerConnection { ice_servers } => {
            if let Err(e) = webrtc::create_bootstrap_pc(shared, &ice_servers) {
                log(&format!("[antenna] create bootstrap PC error: {e:?}"));
            }
        }
        Output::CreateBootstrapDataChannel { label } => {
            webrtc::create_bootstrap_data_channel(shared, &label);
        }
        Output::CreateBootstrapOffer => {
            webrtc::spawn_bootstrap_offer(shared);
        }
        Output::SetBootstrapLocalDescription { sdp, is_offer } => {
            webrtc::spawn_set_bootstrap_local_desc(shared, sdp, is_offer);
        }
        Output::SetBootstrapRemoteDescription { sdp, is_offer } => {
            webrtc::spawn_set_bootstrap_remote_desc(shared, sdp, is_offer);
        }
        Output::CreateBootstrapAnswer => {
            webrtc::spawn_bootstrap_answer(shared);
        }
        Output::InviteReady(invite) => {
            let inner = shared.borrow();
            if let Some(cb) = &inner.on_invite_ready {
                if let Ok(json) = serde_json::to_string(&invite) {
                    cb(json);
                }
            }
        }
        Output::AnswerReady(answer) => {
            let inner = shared.borrow();
            if let Some(cb) = &inner.on_answer_ready {
                if let Ok(json) = serde_json::to_string(&answer) {
                    cb(json);
                }
            }
        }
        Output::CreatePeerConnection {
            remote,
            ice_servers,
        } => {
            if let Err(e) = webrtc::create_peer_connection(shared, &remote, &ice_servers) {
                log(&format!("[antenna] create PC error: {e:?}"));
            }
        }
        Output::CreateDataChannel { remote, label } => {
            webrtc::create_data_channel(shared, &remote, &label);
        }
        Output::CreateOffer { remote } => {
            webrtc::spawn_create_offer(shared, &remote);
        }
        Output::CreateAnswer { remote } => {
            webrtc::spawn_create_answer(shared, &remote);
        }
        Output::SetLocalDescription {
            remote,
            sdp,
            is_offer,
        } => {
            webrtc::spawn_set_description(shared, &remote, sdp, is_offer, true);
        }
        Output::SetRemoteDescription {
            remote,
            sdp,
            is_offer,
        } => {
            webrtc::spawn_set_description(shared, &remote, sdp, is_offer, false);
        }
        Output::AddIceCandidate { remote, candidate } => {
            webrtc::spawn_add_ice_candidate(shared, &remote, candidate);
        }
        Output::ClosePeerConnection { remote } => {
            webrtc::close_peer(shared, &remote);
        }
        Output::SendRelay { to, msg } => {
            send_relay(shared, &to, &msg);
        }
        Output::DeliverMessage { from, data } => {
            let inner = shared.borrow();
            if let Some(cb) = &inner.on_message {
                cb(&from, &data);
            }
        }
        Output::PeerConnected { peer_id } => {
            shared.borrow_mut().connected_peers.insert(peer_id.clone());
            // Clone callback ref to avoid borrow conflict
            let cb = shared.borrow().on_peer_connected.as_ref().map(|_| ());
            if cb.is_some() {
                let inner = shared.borrow();
                if let Some(cb) = &inner.on_peer_connected {
                    cb(&peer_id);
                }
            }
        }
        Output::PeerDisconnected { peer_id } => {
            shared.borrow_mut().connected_peers.remove(&peer_id);
            let cb = shared.borrow().on_peer_disconnected.as_ref().map(|_| ());
            if cb.is_some() {
                let inner = shared.borrow();
                if let Some(cb) = &inner.on_peer_disconnected {
                    cb(&peer_id);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn send_relay(shared: &Shared, to: &PeerId, msg: &RelayMessage) {
    let Ok(json) = serde_json::to_string(msg) else {
        log("[antenna] failed to serialize relay message");
        return;
    };

    let inner = shared.borrow();

    if let Some(ref dc) = inner.bootstrap_dc {
        let is_bootstrap_target =
            inner.mesh.local_id() == &PeerId::master() || to == &PeerId::master();
        if is_bootstrap_target {
            let _ = dc.send_with_str(&json);
            return;
        }
    }

    if let Some(res) = inner.peers.get(to) {
        if let Some(dc) = &res.dc {
            let _ = dc.send_with_str(&json);
        }
    }
}
