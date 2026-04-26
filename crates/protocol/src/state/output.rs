use crate::{MsgPayload, PeerID, UserMsgPayload, handshake::HandshakeOutput};

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

    ///
    PeerAppeared { peer: PeerID },

    /// Notify about new peer connected to mesh
    PeerConnected { peer: PeerID },

    /// Notify about peer that gracefully left the mesh
    PeerDisconnected { peer: PeerID },

    /// Notify about peer lost due to connection failure (abrupt, candidate for reconnect)
    PeerLost { peer: PeerID },

    /// All relay handshakes complete — this peer is fully meshed and may send messages
    Available,

    /// Mesh has no connected peers — this peer can no longer send messages
    Unavailable,

    /// Local node sent disconnect notices to all peers; driver must close all connections
    Disconnecting,
}
