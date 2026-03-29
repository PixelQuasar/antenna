use crate::{TransportInput, mesh::PeerID};

#[derive(Debug, PartialEq, Eq)]
pub enum RelayPayload {
    /// Event notifying that new peer joined
    PeerJoined { peer: PeerID },

    /// Event notifying that peer left the mesh
    PeerLeft { peer: PeerID },

    /// Forwarded transport event for a peer you're not directly connected to yet
    TransportForward {
        target: PeerID,
        event: TransportInput,
    },
}
