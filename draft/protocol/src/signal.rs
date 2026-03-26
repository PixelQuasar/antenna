use serde::{Deserialize, Serialize};

use crate::PeerId;

/// ICE server configuration (STUN/TURN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

impl IceServer {
    /// Default public Google STUN server.
    pub fn default_stun() -> Self {
        Self {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            username: None,
            credential: None,
        }
    }
}

/// An invite blob that the master generates and shares out-of-band.
///
/// Contains the master's SDP offer with all ICE candidates gathered
/// ("vanilla ICE"), so the joiner only needs to produce an answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    /// The master's SDP offer (with ICE candidates baked in).
    pub sdp: String,
    /// ICE servers the joiner should use.
    pub ice_servers: Vec<IceServer>,
}

/// An answer blob that the joiner generates and shares back to the master.
///
/// Contains the joiner's SDP answer with all ICE candidates gathered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// The joiner's SDP answer (with ICE candidates baked in).
    pub sdp: String,
}

/// Messages relayed between peers over the master's data channels.
///
/// Once a joiner is connected to the master, the master acts as a signaling
/// relay. These messages are sent over data channels (not WebSocket).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelayMessage {
    /// Master → Joiner: you have been assigned this peer id.
    Welcome { peer_id: PeerId },

    /// Master → Joiner: a new peer joined the mesh.
    PeerJoined { peer_id: PeerId },

    /// Master → Joiner: a peer left the mesh.
    PeerLeft { peer_id: PeerId },

    /// Peer → Peer (relayed via master): SDP offer for a new mesh link.
    Offer {
        from: PeerId,
        to: PeerId,
        sdp: String,
    },

    /// Peer → Peer (relayed via master): SDP answer.
    Answer {
        from: PeerId,
        to: PeerId,
        sdp: String,
    },

    /// Peer → Peer (relayed via master): ICE candidate.
    IceCandidate {
        from: PeerId,
        to: PeerId,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    },
}
