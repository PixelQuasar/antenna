use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{PeerID, SignalingPayload};

/// Marker trait for application message types carried over the mesh.
///
/// Auto-implemented for any `Serialize + DeserializeOwned + Clone` — users
/// don't implement it manually.
pub trait UserMsgPayload: Serialize + DeserializeOwned + Clone {}

impl<T> UserMsgPayload for T where T: Serialize + DeserializeOwned + Clone {}

/// Inner payload of a relay-signaling frame between two peers via an intermediary.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RelayPayload {
    /// Invitation to start a relay handshake with the given peer.
    InitConnect(PeerID),
    /// Forwarded SDP offer.
    Offer(SignalingPayload),
    /// Forwarded SDP answer.
    Answer(SignalingPayload),
}

/// Every frame that crosses a data channel.
///
/// `User` is the only variant that surfaces to the application; the rest
/// are protocol-internal and consumed by [`crate::MeshNodeFSM`].
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(bound(serialize = "Msg: Serialize", deserialize = "Msg: DeserializeOwned"))]
pub enum MsgPayload<Msg: UserMsgPayload> {
    /// Application payload sent via `Peer::send` / `Peer::broadcast`.
    User(Msg),
    /// "Forward this to `dst`" — sent to a relay intermediary.
    RelaySignalingTo { dst: PeerID, data: RelayPayload },
    /// "Here is a frame from `src`" — emitted by a relay intermediary to the destination.
    RelaySignalingFrom { src: PeerID, data: RelayPayload },
    /// Graceful-disconnect notice broadcast by a leaving peer.
    Disconnect,
}
