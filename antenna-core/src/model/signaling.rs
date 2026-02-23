use crate::model::peer::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// Set of possible types of signaling messages. Defines the signaling protocol of antenna SDK.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", content = "d")]
pub enum SignalMessage {
    /// Contains STUN/TURN server urls and creds. Sent by the server to the client immediately after connection.
    IceConfig { ice_servers: Vec<IceServerConfig> },
    /// Client-joining-room message. contains room id what client wants to join.
    Join { room: String },
    /// Sent when client initiates peer connection, contains its own SDP string.
    Offer { sdp: String },
    /// Sent in response of clients offer, contains server SDP string.
    Answer { sdp: String },
    /// sent by both sides to discover new network paths.
    IceCandidate { candidate: String },
    /// Sent by the server to confirm the client has successfully joined the room. Contains new session id (peer_id)
    Welcome { peer_id: PeerId },
}
