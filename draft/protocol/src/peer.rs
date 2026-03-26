use crate::PeerId;

/// Lifecycle phase of a single peer-to-peer WebRTC connection.
///
/// ```text
///   New ──► CreatingOffer ──► OfferSent ──► Connected
///   New ──► OfferReceived ──► AnswerSent ──► Connected
///   * ──► Disconnected
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// We know about this peer but haven't started negotiation yet.
    New,
    /// We are the offerer — creating a local offer.
    CreatingOffer,
    /// Our offer has been sent; waiting for an answer.
    OfferSent,
    /// We received a remote offer; need to create an answer.
    OfferReceived,
    /// Our answer has been sent; waiting for the data channel to open.
    AnswerSent,
    /// DataChannel is open — ready to exchange data.
    Connected,
    /// Connection was closed or failed.
    Disconnected,
}

/// Parsed ICE candidate data (platform-agnostic).
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

/// State tracked for each remote peer in the mesh.
#[derive(Debug)]
pub struct PeerState {
    pub id: PeerId,
    pub phase: Phase,
    /// Whether *we* are the "polite" peer (our id < remote id).
    /// The polite peer yields its own offer on glare.
    pub polite: bool,
    /// ICE candidates received before the remote description was set.
    pub pending_candidates: Vec<IceCandidate>,
}

impl PeerState {
    pub fn new(id: PeerId, polite: bool) -> Self {
        Self {
            id,
            phase: Phase::New,
            polite,
            pending_candidates: Vec::new(),
        }
    }

    /// Whether the remote description has been set (safe to add ICE candidates).
    pub fn has_remote_description(&self) -> bool {
        matches!(
            self.phase,
            Phase::OfferSent | Phase::AnswerSent | Phase::Connected
        )
    }
}
