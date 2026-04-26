use serde::{Deserialize, Serialize};

use crate::PeerID;

/// Scheduled action identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Scheduled {
    ReconnectAttempt { peer: PeerID },
}
