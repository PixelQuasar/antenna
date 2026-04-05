use crate::{MsgPayload, UserMsgPayload, handshake::HandshakeOutput, mesh::PeerID};

/// Common event that client FSM sends
#[derive(Debug, Clone)]
pub enum Output<Msg: UserMsgPayload> {
    /// Handshake event
    Handshake {
        event: HandshakeOutput,
        peer: PeerID,
    },

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

    /// Notify about new peer disconnected to mesh
    PeerDisconnected { peer: PeerID },
}
