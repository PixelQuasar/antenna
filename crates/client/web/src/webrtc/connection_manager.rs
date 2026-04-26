use std::cell::RefCell;

use crate::{DataChannelManager, PeerConnectionManager};
use antenna_client_shared::IceServerConfig;
use anyhow::Result;
use wasm_bindgen::{JsValue, closure::Closure};

type OnDataChannelCb = Closure<dyn FnMut(JsValue)>;

pub struct ConnectionManager {
    pc: PeerConnectionManager,
    dc: RefCell<Option<DataChannelManager>>,
    ondatachannel_cb: RefCell<Option<OnDataChannelCb>>,
}

impl ConnectionManager {
    pub fn new_host(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers)?;
        let dc = DataChannelManager::new(pc.peer_connection(), "data");
        Ok(Self {
            pc,
            dc: RefCell::new(Some(dc)),
            ondatachannel_cb: RefCell::new(None),
        })
    }

    pub fn new_joiner(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers)?;
        Ok(Self {
            pc,
            dc: RefCell::new(None),
            ondatachannel_cb: RefCell::new(None),
        })
    }

    pub async fn create_offer(&self) -> Result<String> {
        let sdp = self.pc.create_offer().await?;
        self.pc.set_local_description(&sdp, true).await?;
        self.pc.fetch_sdp().await
    }

    pub async fn create_answer(&self, offer_sdp: &str) -> Result<String> {
        self.pc.set_remote_description(offer_sdp, true).await?;
        let sdp = self.pc.create_answer().await?;
        self.pc.set_local_description(&sdp, false).await?;
        self.pc.fetch_sdp().await
    }

    pub async fn accept_answer(&self, answer_sdp: &str) -> Result<()> {
        self.pc.set_remote_description(answer_sdp, false).await
    }

    pub fn set_dc(&self, dc: DataChannelManager) {
        *self.dc.borrow_mut() = Some(dc);
    }

    pub fn dc(&self) -> &RefCell<Option<DataChannelManager>> {
        &self.dc
    }

    pub fn rtc_peer_connection(&self) -> &web_sys::RtcPeerConnection {
        self.pc.peer_connection()
    }

    pub fn store_ondatachannel_closure(&self, cb: Closure<dyn FnMut(JsValue)>) {
        *self.ondatachannel_cb.borrow_mut() = Some(cb);
    }

    pub fn setup_on_ice_state_change<F>(&self, callback: F)
    where
        F: FnMut(web_sys::RtcIceConnectionState) + 'static,
    {
        self.pc.setup_on_ice_state_change(callback);
    }

    pub fn close(&self) {
        self.pc.peer_connection().set_ondatachannel(None);
        *self.ondatachannel_cb.borrow_mut() = None;
        if let Some(dc) = self.dc.borrow().as_ref() {
            dc.close();
        }
        self.pc.close();
    }
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        self.close();
    }
}
