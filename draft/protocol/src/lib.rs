//! # antenna-protocol
//!
//! Sans-IO protocol layer for a serverless P2P WebRTC full-mesh network.
//!
//! This crate contains **zero platform dependencies** — no WebSocket, no
//! WebRTC, no browser APIs. It defines:
//!
//! - The signaling protocol ([`signal`]) — invite/answer for bootstrap,
//!   relay messages for mesh expansion
//! - Per-peer connection state ([`peer::PeerState`])
//! - A pure state machine ([`mesh::Mesh`]) that consumes [`mesh::Input`]
//!   events and produces [`mesh::Output`] commands
//!
//! ## Serverless Architecture
//!
//! The first connection is bootstrapped via copy-paste: the master generates
//! an invite (SDP offer with all ICE candidates), the joiner pastes it and
//! produces an answer. After the bootstrap data channel opens, the master
//! acts as a signaling relay for subsequent peers to form a full mesh.

pub mod mesh;
pub mod peer;
pub mod peer_id;
pub mod signal;

pub use mesh::{Input, Mesh, Output, Role};
pub use peer::{IceCandidate, PeerState, Phase};
pub use peer_id::PeerId;
pub use signal::{Answer, IceServer, Invite, RelayMessage};
