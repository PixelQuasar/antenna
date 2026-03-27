use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys;

use crate::utils::{IceServerConfig, async_callback};

pub struct PeerConnectionManager {
    peer_connection: web_sys::RtcPeerConnection,
}

impl PeerConnectionManager {
    pub fn from_ice_config(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let peer_connection = web_sys::RtcPeerConnection::new_with_configuration(
            &IceServerConfig::build_rtc_config(ice_servers),
        )
        .map_err(|e| anyhow!("Failed to create PeerConnection: {:?}", e))?;

        Ok(Self { peer_connection })
    }

    pub fn peer_connection(&self) -> &web_sys::RtcPeerConnection {
        &self.peer_connection
    }

    pub async fn wait_for_ice_gathering_complete(&self) -> Result<String> {
        // Early return if already complete
        if self.peer_connection.ice_gathering_state()
            == web_sys::RtcIceGatheringState::Complete
        {
            if let Some(desc) = self.peer_connection.local_description() {
                return Ok(desc.sdp());
            }
        }

        async_callback(|mut resolve| {
            let cb_peer_connection = self.peer_connection.clone();
            let cb = Closure::wrap(Box::new(move |_evt: JsValue| {
                if cb_peer_connection.ice_gathering_state()
                    != web_sys::RtcIceGatheringState::Complete
                {
                    return;
                }
                if let Some(desc) = cb_peer_connection.local_description() {
                    resolve(desc.sdp());
                }
            }));

            self.peer_connection
                .set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
            cb.forget();
        })
        .await
        .ok_or_else(|| anyhow!("ICE gathering callback failed"))
    }

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
