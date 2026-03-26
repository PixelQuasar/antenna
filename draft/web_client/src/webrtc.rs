use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use antenna_protocol::{IceCandidate, IceServer, Input, PeerId};

use crate::driver::{Shared, feed_input};
use crate::log;

/// Browser-side resources for a single remote peer.
pub struct PeerResources {
    pub pc: web_sys::RtcPeerConnection,
    pub dc: Option<web_sys::RtcDataChannel>,
}

// ---------------------------------------------------------------------------
// RTCPeerConnection creation
// ---------------------------------------------------------------------------

pub fn create_peer_connection(
    shared: &Shared,
    remote: &PeerId,
    ice_servers: &[IceServer],
) -> Result<(), JsValue> {
    let rtc_config = build_rtc_config(ice_servers);
    let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config)?;

    attach_ice_candidate_handler(shared, remote, &pc);
    attach_ice_state_handler(shared, remote, &pc);
    attach_datachannel_handler(shared, remote, &pc);

    let resources = PeerResources { pc, dc: None };
    shared.borrow_mut().peers.insert(remote.clone(), resources);
    Ok(())
}

/// Create the bootstrap peer connection (master↔first joiner).
/// This is stored separately from mesh peers.
pub fn create_bootstrap_pc(shared: &Shared, ice_servers: &[IceServer]) -> Result<(), JsValue> {
    let rtc_config = build_rtc_config(ice_servers);
    let pc = web_sys::RtcPeerConnection::new_with_configuration(&rtc_config)?;

    // ICE candidate handler — for bootstrap we use vanilla ICE (gather all first),
    // so we watch for gathering complete rather than individual candidates.
    {
        let s = shared.clone();
        let pc_clone = pc.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_evt: JsValue| {
            let state = pc_clone.ice_gathering_state();
            if state == web_sys::RtcIceGatheringState::Complete {
                // Get the local description with all candidates baked in
                if let Some(desc) = pc_clone.local_description() {
                    let sdp = desc.sdp();
                    let is_offer = desc.type_() == web_sys::RtcSdpType::Offer;
                    if is_offer {
                        feed_input(&s, Input::BootstrapIceGatheringComplete { sdp });
                    } else {
                        feed_input(&s, Input::BootstrapAnswerIceComplete { sdp });
                    }
                }
            }
        }));
        pc.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    // ondatachannel (for the joiner side — receives the master's data channel)
    {
        let s = shared.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let evt: web_sys::RtcDataChannelEvent = evt.unchecked_into();
            let dc = evt.channel();
            setup_data_channel(&s, &PeerId::master(), &dc);
            let mut inner = s.borrow_mut();
            inner.bootstrap_dc = Some(dc);
        }));
        pc.set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    shared.borrow_mut().bootstrap_pc = Some(pc);
    Ok(())
}

fn build_rtc_config(ice_servers: &[IceServer]) -> web_sys::RtcConfiguration {
    let config = web_sys::RtcConfiguration::new();
    let arr = js_sys::Array::new();

    for server in ice_servers {
        let ice = web_sys::RtcIceServer::new();
        let urls = js_sys::Array::new();
        for u in &server.urls {
            urls.push(&JsValue::from_str(u));
        }
        ice.set_urls(&urls);
        if let Some(username) = &server.username {
            ice.set_username(username);
        }
        if let Some(credential) = &server.credential {
            ice.set_credential(credential);
        }
        arr.push(&ice);
    }

    config.set_ice_servers(&arr);
    config
}

fn attach_ice_candidate_handler(shared: &Shared, remote: &PeerId, pc: &web_sys::RtcPeerConnection) {
    let s = shared.clone();
    let remote = remote.clone();
    let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
        let evt: web_sys::RtcPeerConnectionIceEvent = evt.unchecked_into();
        let Some(candidate) = evt.candidate() else {
            return;
        };
        feed_input(
            &s,
            Input::LocalIceCandidate {
                remote: remote.clone(),
                candidate: candidate.candidate(),
                sdp_mid: candidate.sdp_mid(),
                sdp_m_line_index: candidate.sdp_m_line_index(),
            },
        );
    }));
    pc.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
}

fn attach_ice_state_handler(shared: &Shared, remote: &PeerId, pc: &web_sys::RtcPeerConnection) {
    let s = shared.clone();
    let remote = remote.clone();
    let pc_clone = pc.clone();
    let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
        let state = format!("{:?}", pc_clone.ice_connection_state());
        feed_input(
            &s,
            Input::IceConnectionStateChange {
                remote: remote.clone(),
                state,
            },
        );
    }));
    pc.set_oniceconnectionstatechange(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
}

fn attach_datachannel_handler(shared: &Shared, remote: &PeerId, pc: &web_sys::RtcPeerConnection) {
    let s = shared.clone();
    let remote = remote.clone();
    let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
        let evt: web_sys::RtcDataChannelEvent = evt.unchecked_into();
        let dc = evt.channel();
        setup_data_channel(&s, &remote, &dc);
        let mut inner = s.borrow_mut();
        if let Some(res) = inner.peers.get_mut(&remote) {
            res.dc = Some(dc);
        }
    }));
    pc.set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
}

// ---------------------------------------------------------------------------
// DataChannel setup
// ---------------------------------------------------------------------------

pub fn setup_data_channel(shared: &Shared, remote: &PeerId, dc: &web_sys::RtcDataChannel) {
    dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);

    // onopen
    {
        let s = shared.clone();
        let remote = remote.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
            feed_input(
                &s,
                Input::DataChannelOpen {
                    remote: remote.clone(),
                },
            );
        }));
        dc.set_onopen(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    // onclose
    {
        let s = shared.clone();
        let remote = remote.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
            feed_input(
                &s,
                Input::DataChannelClosed {
                    remote: remote.clone(),
                },
            );
        }));
        dc.set_onclose(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    // onmessage
    {
        let s = shared.clone();
        let remote = remote.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let evt: web_sys::MessageEvent = evt.unchecked_into();
            // Try ArrayBuffer first, then string
            if let Ok(ab) = evt.data().dyn_into::<js_sys::ArrayBuffer>() {
                let bytes = js_sys::Uint8Array::new(&ab).to_vec();
                feed_input(
                    &s,
                    Input::DataReceived {
                        from: remote.clone(),
                        data: bytes,
                    },
                );
            } else if let Some(text) = evt.data().as_string() {
                feed_input(
                    &s,
                    Input::DataReceived {
                        from: remote.clone(),
                        data: text.into_bytes(),
                    },
                );
            }
        }));
        dc.set_onmessage(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }
}

// ---------------------------------------------------------------------------
// Bootstrap operations
// ---------------------------------------------------------------------------

pub fn create_bootstrap_data_channel(shared: &Shared, label: &str) {
    let inner = shared.borrow();
    let Some(pc) = &inner.bootstrap_pc else {
        return;
    };
    let dc = pc.create_data_channel(label);
    drop(inner);

    setup_data_channel(shared, &PeerId::from("bootstrap-remote"), &dc);

    let mut inner = shared.borrow_mut();
    inner.bootstrap_dc = Some(dc);
}

pub fn spawn_bootstrap_offer(shared: &Shared) {
    let pc = {
        let inner = shared.borrow();
        inner.bootstrap_pc.clone()
    };
    let Some(pc) = pc else { return };

    let s = shared.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(offer) = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await else {
            log("[antenna] create bootstrap offer failed");
            return;
        };
        let Some(sdp) = js_sys::Reflect::get(&offer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
        else {
            log("[antenna] bootstrap offer has no sdp");
            return;
        };
        feed_input(&s, Input::BootstrapOfferCreated { sdp });
    });
}

pub fn spawn_bootstrap_answer(shared: &Shared) {
    let pc = {
        let inner = shared.borrow();
        inner.bootstrap_pc.clone()
    };
    let Some(pc) = pc else { return };

    let s = shared.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(answer) = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await else {
            log("[antenna] create bootstrap answer failed");
            return;
        };
        let Some(sdp) = js_sys::Reflect::get(&answer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
        else {
            log("[antenna] bootstrap answer has no sdp");
            return;
        };
        feed_input(&s, Input::BootstrapAnswerCreated { sdp });
    });
}

pub fn spawn_set_bootstrap_local_desc(shared: &Shared, sdp: String, is_offer: bool) {
    let pc = {
        let inner = shared.borrow();
        inner.bootstrap_pc.clone()
    };
    let Some(pc) = pc else { return };

    wasm_bindgen_futures::spawn_local(async move {
        let sdp_type = if is_offer {
            web_sys::RtcSdpType::Offer
        } else {
            web_sys::RtcSdpType::Answer
        };
        let desc = web_sys::RtcSessionDescriptionInit::new(sdp_type);
        desc.set_sdp(&sdp);
        if let Err(e) = wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc)).await
        {
            log(&format!("[antenna] set bootstrap local desc error: {e:?}"));
        }
    });
}

pub fn spawn_set_bootstrap_remote_desc(shared: &Shared, sdp: String, is_offer: bool) {
    let pc = {
        let inner = shared.borrow();
        inner.bootstrap_pc.clone()
    };
    let Some(pc) = pc else { return };

    wasm_bindgen_futures::spawn_local(async move {
        let sdp_type = if is_offer {
            web_sys::RtcSdpType::Offer
        } else {
            web_sys::RtcSdpType::Answer
        };
        let desc = web_sys::RtcSessionDescriptionInit::new(sdp_type);
        desc.set_sdp(&sdp);
        if let Err(e) = wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&desc)).await
        {
            log(&format!("[antenna] set bootstrap remote desc error: {e:?}"));
        }
    });
}

// ---------------------------------------------------------------------------
// Mesh peer operations
// ---------------------------------------------------------------------------

pub fn create_data_channel(shared: &Shared, remote: &PeerId, label: &str) {
    let remote = remote.clone();
    let mut inner = shared.borrow_mut();
    let Some(res) = inner.peers.get_mut(&remote) else {
        return;
    };
    let dc = res.pc.create_data_channel(label);
    let dc_clone = dc.clone();
    drop(inner);

    setup_data_channel(shared, &remote, &dc_clone);

    let mut inner = shared.borrow_mut();
    if let Some(res) = inner.peers.get_mut(&remote) {
        res.dc = Some(dc);
    }
}

pub fn spawn_create_offer(shared: &Shared, remote: &PeerId) {
    let pc = {
        let inner = shared.borrow();
        inner.peers.get(remote).map(|r| r.pc.clone())
    };
    let Some(pc) = pc else { return };

    let s = shared.clone();
    let remote = remote.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(offer) = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await else {
            log("[antenna] create offer failed");
            return;
        };
        let Some(sdp) = js_sys::Reflect::get(&offer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
        else {
            log("[antenna] offer has no sdp");
            return;
        };
        feed_input(&s, Input::LocalOfferCreated { remote, sdp });
    });
}

pub fn spawn_create_answer(shared: &Shared, remote: &PeerId) {
    let pc = {
        let inner = shared.borrow();
        inner.peers.get(remote).map(|r| r.pc.clone())
    };
    let Some(pc) = pc else { return };

    let s = shared.clone();
    let remote = remote.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(answer) = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await else {
            log("[antenna] create answer failed");
            return;
        };
        let Some(sdp) = js_sys::Reflect::get(&answer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
        else {
            log("[antenna] answer has no sdp");
            return;
        };
        feed_input(&s, Input::LocalAnswerCreated { remote, sdp });
    });
}

pub fn spawn_set_description(
    shared: &Shared,
    remote: &PeerId,
    sdp: String,
    is_offer: bool,
    is_local: bool,
) {
    let pc = {
        let inner = shared.borrow();
        inner.peers.get(remote).map(|r| r.pc.clone())
    };
    let Some(pc) = pc else { return };

    wasm_bindgen_futures::spawn_local(async move {
        let sdp_type = if is_offer {
            web_sys::RtcSdpType::Offer
        } else {
            web_sys::RtcSdpType::Answer
        };
        let desc = web_sys::RtcSessionDescriptionInit::new(sdp_type);
        desc.set_sdp(&sdp);

        let promise = if is_local {
            pc.set_local_description(&desc)
        } else {
            pc.set_remote_description(&desc)
        };

        if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
            let kind = if is_local { "local" } else { "remote" };
            log(&format!("[antenna] set {kind} desc error: {e:?}"));
        }
    });
}

pub fn spawn_add_ice_candidate(shared: &Shared, remote: &PeerId, candidate: IceCandidate) {
    let pc = {
        let inner = shared.borrow();
        inner.peers.get(remote).map(|r| r.pc.clone())
    };
    let Some(pc) = pc else { return };

    let init = web_sys::RtcIceCandidateInit::new(&candidate.candidate);
    if let Some(mid) = &candidate.sdp_mid {
        init.set_sdp_mid(Some(mid));
    }
    if let Some(idx) = candidate.sdp_m_line_index {
        init.set_sdp_m_line_index(Some(idx));
    }

    let promise = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&init));
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
            log(&format!("[antenna] add ICE error: {e:?}"));
        }
    });
}

pub fn close_peer(shared: &Shared, remote: &PeerId) {
    let mut inner = shared.borrow_mut();
    if let Some(res) = inner.peers.remove(remote) {
        if let Some(dc) = &res.dc {
            dc.close();
        }
        res.pc.close();
    }
}
