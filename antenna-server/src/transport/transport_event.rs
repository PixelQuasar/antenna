use antenna_core::PeerId;
use bytes::Bytes;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;

pub enum TransportEvent {
    DataChannelReady(PeerId, Arc<RTCDataChannel>),
    Disconnected(PeerId),
    Message(PeerId, Bytes),
    CandidateGenerated(PeerId, String),
}
