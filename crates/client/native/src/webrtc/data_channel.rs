use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::peer_connection::RTCPeerConnection;

use antenna_protocol::UserMsgPayload;

/// RTCDataChannel wrapper
pub struct DataChannelManager {
    dc: Arc<RTCDataChannel>,
}

impl DataChannelManager {
    pub async fn new(pc: &RTCPeerConnection, label: &str) -> Result<Self> {
        let dc = pc
            .create_data_channel(label, None)
            .await
            .map_err(|e| anyhow!("create_data_channel failed: {e}"))?;
        Ok(Self { dc })
    }

    pub fn from_existing(dc: Arc<RTCDataChannel>) -> Self {
        Self { dc }
    }

    pub fn setup_on_open<F>(&self, mut on_open: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.dc.on_open(Box::new(move || {
            on_open();
            Box::pin(async {})
        }));
    }

    pub fn setup_on_message<F>(&self, mut on_message: F)
    where
        F: FnMut(Vec<u8>) + Send + Sync + 'static,
    {
        self.dc.on_message(Box::new(move |msg: DataChannelMessage| {
            on_message(msg.data.to_vec());
            Box::pin(async {})
        }));
    }

    pub fn setup_on_close<F>(&self, mut on_close: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.dc.on_close(Box::new(move || {
            on_close();
            Box::pin(async {})
        }));
    }

    pub async fn send_data<Msg: UserMsgPayload>(&self, data: &Msg) -> Result<()> {
        let bytes =
            serde_json::to_vec(data).map_err(|e| anyhow!("Failed to serialize message: {e}"))?;
        self.dc
            .send(&Bytes::from(bytes))
            .await
            .map_err(|e| anyhow!("Failed to send data: {e}"))?;
        Ok(())
    }

    pub async fn close(&self) {
        let _ = self.dc.close().await;
    }
}
