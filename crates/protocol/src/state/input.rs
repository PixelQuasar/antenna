use crate::{
    HandshakeMode, HandshakeStrategy, PeerID, Scheduled, UserMsgPayload, handshake::HandshakeInput,
    state::MsgPayload,
};

/// Common event that client FSM receives
#[derive(Debug, Clone)]
pub enum Input<Msg: UserMsgPayload> {
    /// Initiate a handshake with a known peer ID, choosing role and signaling mode.
    InitHandshake {
        with: PeerID,
        mode: HandshakeMode,
        strategy: HandshakeStrategy,
    },

    /// Bootstrap host initializes an open offer without knowing the joiner's ID yet.
    InitOpenOffer,

    /// Bootstrap host's SDP offer was created by WebRTC.
    OpenOfferCreated(String),

    /// Receive handshake event from a known peer
    Handshake { from: PeerID, event: HandshakeInput },

    /// Receive abstract message
    MessageReceived {
        peer_from: PeerID,
        data: MsgPayload<Msg>,
    },

    /// Send abstract message to other peer
    Send {
        peer_to: PeerID,
        data: MsgPayload<Msg>,
    },

    /// Send abstract message to all peers
    Broadcast { data: MsgPayload<Msg> },

    /// Receive peer leaving message
    PeerLeaving { peer: PeerID },

    /// The local node initiates departure from the mesh
    Leave,

    /// Driver-scheduled timer expired; FSM dispatches by kind.
    TimerFired { kind: Scheduled },
}
