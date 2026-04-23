use crate::{DataChannelManager, IceServerConfig, PeerConnectionManager};
use anyhow::Result;
use std::cell::RefCell;

pub struct ConnectionManager {
    pc: PeerConnectionManager,
    dc: RefCell<Option<DataChannelManager>>,
}

impl ConnectionManager {
    pub fn new_host(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers)?;
        let dc = DataChannelManager::new(pc.peer_connection(), "data");
        Ok(Self {
            pc,
            dc: RefCell::new(Some(dc)),
        })
    }

    pub fn new_joiner(ice_servers: &[IceServerConfig]) -> Result<Self> {
        let pc = PeerConnectionManager::from_ice_config(ice_servers)?;
        Ok(Self {
            pc,
            dc: RefCell::new(None),
        })
    }

    pub async fn create_offer(&self) -> Result<String> {
        let sdp = self.pc.create_offer().await?;
        self.pc.set_local_description(&sdp, true).await?;
        self.pc.wait_for_ice_gathering_complete().await
    }

    pub async fn create_answer(&self, offer_sdp: &str) -> Result<String> {
        self.pc.set_remote_description(offer_sdp, true).await?;
        let sdp = self.pc.create_answer().await?;
        self.pc.set_local_description(&sdp, false).await?;
        self.pc.wait_for_ice_gathering_complete().await
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

    pub fn close(&self) {
        self.dc.borrow().as_ref().map(|dc| dc.close());
        self.pc.close();
    }
}
