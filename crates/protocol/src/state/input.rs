use crate::{UserMsgPayload, handshake::HandshakeInput, mesh::PeerID};

/// Common event that client FSM receives
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input<Msg: UserMsgPayload> {
    /// Reveice handshake event
    Handshake { event: HandshakeInput, from: PeerID },

    /// Receive abstract message
    MessageReceived { peer_from: PeerID, data: Msg },

    /// Send abstract message to other peer
    Send { peer_to: PeerID, data: Msg },

    /// Send abstract message to all peers
    Broadcast { data: Msg },

    /// Receive peer leaving message
    PeerLeaving { peer: PeerID },
}
