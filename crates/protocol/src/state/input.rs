use crate::{
    HandshakeMode, HandshakeStrategy, UserMsgPayload, handshake::HandshakeInput, mesh::PeerID,
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

    /// Reveice handshake event
    Handshake { event: HandshakeInput, from: PeerID },

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
