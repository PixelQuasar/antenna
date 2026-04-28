use crate::{
    MsgPayload, PeerID, Scheduled, SignalingPayload, UserMsgPayload, handshake::HandshakeOutput,
};

/// Common event that client FSM sends
#[derive(Debug, Clone)]
pub enum Output<Msg: UserMsgPayload> {
    /// Handshake event for a known peer
    Handshake {
        peer: PeerID,
        event: HandshakeOutput,
    },

    /// Bootstrap host: create an open offer (joiner peer ID not yet known)
    InitOpenOffer,

    /// Bootstrap host: open offer SDP is ready to be shared out-of-band with a joiner
    OfferReady(SignalingPayload),

    /// Bootstrap joiner: SDP answer is ready to be shared out-of-band with the host
    AnswerReady(SignalingPayload),

    /// Send message to other peer in mesh
    SendMessage {
        peer_to: PeerID,
        data: MsgPayload<Msg>,
    },

    /// Initiate receiving message from any outer sender
    ReceiveMessage {
        peer_from: PeerID,
        data: MsgPayload<Msg>,
    },

    /// Notify about new peer connected to mesh
    PeerConnected { peer: PeerID },

    /// Notify about peer that gracefully left the mesh
    PeerDisconnected { peer: PeerID },

    /// Notify about peer lost due to connection failure (abrupt, candidate for reconnect)
    PeerLost { peer: PeerID },

    /// Node-level status: first peer reached `Connected` (FSMState: Init → Connected).
    Connected,

    /// Node-level status: all relay handshakes complete (FSMState: Connected → Available).
    Available,

    /// Node-level status: a relay handshake is in progress or no peers are connected
    /// (FSMState: Available → Connected, or any → Init).
    Unavailable,

    /// Local node sent disconnect notices to all peers; driver must close all connections
    /// (FSMState: anything → Left).
    Disconnecting,

    /// Driver should schedule a timer
    ScheduleTimer { kind: Scheduled, after_ms: u64 },
}
