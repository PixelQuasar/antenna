use antenna_core::PeerId;
use bytes::Bytes;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::track::track_remote::TrackRemote;

pub enum TransportEvent {
    DataChannelReady(PeerId, Arc<RTCDataChannel>),
    Track(PeerId, Arc<TrackRemote>),
    Disconnected(PeerId),
    Message(PeerId, Bytes),
    CandidateGenerated(PeerId, String),
}
