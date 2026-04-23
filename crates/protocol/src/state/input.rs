use crate::{
    HandshakeMode, HandshakeStrategy, PeerID, UserMsgPayload, handshake::HandshakeInput,
    state::MsgPayload,
};

/// Common event that client FSM receives
#[derive(Debug, Clone)]
pub enum Input<Msg: UserMsgPayload> {
    ///
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
}
