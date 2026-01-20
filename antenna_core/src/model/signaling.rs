use crate::model::peer::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", content = "d")]
pub enum SignalMessage {
    Join {
        room: String,
        token: Option<String>,
    },
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    },
    Welcome {
        peer_id: PeerId,
    },
}
