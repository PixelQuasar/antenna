use crate::{TransportInput, mesh::PeerID};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RelayPayload {
    /// Event notifying that peer left the mesh
    PeerLeft { peer: PeerID },

    ///
    ConnectionRequest { peer: PeerID },

    /// Forwarded transport event for a peer you're not directly connected to yet
    TransportForward {
        src: PeerID,
        dst: PeerID,
        event: TransportInput,
    },
}
