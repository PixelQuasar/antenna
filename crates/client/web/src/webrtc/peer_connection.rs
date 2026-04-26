use std::cell::RefCell;

use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::utils::{IceServerConfig, async_callback, build_rtc_config};

type IceCb = Closure<dyn FnMut(JsValue)>;
type IceStateCb = Closure<dyn FnMut(JsValue)>;

pub struct PeerConnectionManager {
    peer_connection: web_sys::RtcPeerConnection,
    ice_cb: RefCell<Option<IceCb>>,
    ice_state_cb: RefCell<Option<IceStateCb>>,
}

impl PeerConnectionManager {
    /// creates new peer connection manager from ICE config
    pub fn from_ice_config(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let peer_connection =
            web_sys::RtcPeerConnection::new_with_configuration(&build_rtc_config(ice_servers))
                .map_err(|e| anyhow!("Failed to create PeerConnection: {:?}", e))?;

        Ok(Self {
            peer_connection,
            ice_cb: RefCell::new(None),
            ice_state_cb: RefCell::new(None),
        })
    }

    /// Wires oniceconnectionstatechange and forwards the current ICE connection state to callback
    /// on every state change.
    pub fn setup_on_ice_state_change<F>(&self, mut callback: F)
    where
        F: FnMut(web_sys::RtcIceConnectionState) + 'static,
    {
        let pc = self.peer_connection.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::new(move |_evt: JsValue| {
            callback(pc.ice_connection_state());
        });
        self.peer_connection
            .set_oniceconnectionstatechange(Some(cb.as_ref().unchecked_ref()));
        *self.ice_state_cb.borrow_mut() = Some(cb);
    }

    /// peer connection web object getter
    pub fn peer_connection(&self) -> &web_sys::RtcPeerConnection {
        &self.peer_connection
    }

    /// blocks async flow until ice gathering would be complete and then generates SDP description
    pub async fn fetch_sdp(&self) -> Result<String> {
        if self.peer_connection.ice_gathering_state() == web_sys::RtcIceGatheringState::Complete
            && let Some(desc) = self.peer_connection.local_description()
        {
            return Ok(desc.sdp());
        }

        let result = async_callback(|mut resolve| {
            let pc = self.peer_connection.clone();
            let cb = Closure::wrap(Box::new(move |_evt: JsValue| {
                if pc.ice_gathering_state() != web_sys::RtcIceGatheringState::Complete {
                    return;
                }
                if let Some(desc) = pc.local_description() {
                    resolve(desc.sdp());
                }
            }) as Box<dyn FnMut(JsValue)>);
            self.peer_connection
                .set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
            *self.ice_cb.borrow_mut() = Some(cb);
        })
        .await
        .ok_or_else(|| anyhow!("ICE gathering callback failed"));

        self.peer_connection.set_onicecandidate(None);
        *self.ice_cb.borrow_mut() = None;

        result
    }

    /// Creates sdp offer
    pub async fn create_offer(&self) -> Result<String> {
        let offer = JsFuture::from(self.peer_connection.create_offer())
            .await
            .map_err(|e| anyhow!("Failed to create offer: {:?}", e))?;

        let sdp = js_sys::Reflect::get(&offer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| anyhow!("Offer has no SDP"))?;
        Ok(sdp)
    }

    /// Creates sdp answer
    pub async fn create_answer(&self) -> Result<String> {
        let answer = JsFuture::from(self.peer_connection.create_answer())
            .await
            .map_err(|e| anyhow!("Failed to create answer: {:?}", e))?;

        let sdp = js_sys::Reflect::get(&answer, &"sdp".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| anyhow!("Answer has no SDP"))?;
        Ok(sdp)
    }

    /// Set local sdp description
    pub async fn set_local_description(&self, sdp: &str, is_offer: bool) -> Result<()> {
        let sdp_type = if is_offer {
            web_sys::RtcSdpType::Offer
        } else {
            web_sys::RtcSdpType::Answer
        };

        let desc = web_sys::RtcSessionDescriptionInit::new(sdp_type);
        desc.set_sdp(sdp);

        JsFuture::from(self.peer_connection.set_local_description(&desc))
            .await
            .map_err(|e| anyhow!("Failed to set local description: {:?}", e))?;

        Ok(())
    }

    /// Set remote description
    pub async fn set_remote_description(&self, sdp: &str, is_offer: bool) -> Result<()> {
        let sdp_type = if is_offer {
            web_sys::RtcSdpType::Offer
        } else {
            web_sys::RtcSdpType::Answer
        };

        let desc = web_sys::RtcSessionDescriptionInit::new(sdp_type);
        desc.set_sdp(sdp);

        JsFuture::from(self.peer_connection.set_remote_description(&desc))
            .await
            .map_err(|e| anyhow!("Failed to set remote description: {:?}", e))?;

        Ok(())
    }

    pub fn close(&self) {
        self.peer_connection.close();
    }
}

impl Drop for PeerConnectionManager {
    fn drop(&mut self) {
        self.peer_connection.set_onicecandidate(None);
        self.peer_connection.set_oniceconnectionstatechange(None);
        self.close();
    }
}
