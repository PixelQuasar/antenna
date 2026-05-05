mod host;
mod input;
mod joiner;
mod output;
mod signaling;
mod state;
mod strategy;
mod test;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use host::Host;
pub use input::HandshakeInput;
pub use joiner::Joiner;
pub use output::HandshakeOutput;
pub use signaling::SignalingPayload;
pub use state::HandshakeState;
pub use strategy::{HandshakeStrategy, StrategyFSM};

use crate::PeerID;

/// Per-peer handshake state machine.
///
/// One [`HandshakeFSM`] exists per remote peer in [`crate::MeshNodeFSM`].
/// Dispatches to a [`Host`] or [`Joiner`] strategy depending on which side
/// initiates the SDP exchange.
pub struct HandshakeFSM {
    strategy: HandshakeStrategy,
    fsm: StrategyFSM,
}

impl HandshakeFSM {
    pub fn new(strategy: HandshakeStrategy) -> Self {
        match strategy {
            HandshakeStrategy::Host => Self::host(),
            HandshakeStrategy::Joiner => Self::joiner(),
        }
    }

    /// Host created in relay flow, knows its joiner from the initiation
    pub fn host() -> Self {
        Self {
            strategy: HandshakeStrategy::Host,
            fsm: StrategyFSM::Host(Host::new()),
        }
    }

    pub fn joiner() -> Self {
        Self {
            strategy: HandshakeStrategy::Joiner,
            fsm: StrategyFSM::Joiner(Joiner::new()),
        }
    }

    /// Get current handshake stat
    pub fn state(&self) -> &HandshakeState {
        self.fsm.state()
    }

    pub fn strategy(&self) -> &HandshakeStrategy {
        &self.strategy
    }

    /// Process handshake input and generate handshake output
    pub fn process(&mut self, input: HandshakeInput) -> Result<Option<HandshakeOutput>> {
        self.fsm.process(input)
    }
}

/// How the SDP offer/answer pair is exchanged between two peers.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeMode {
    /// First connection between two strangers — signaling goes through an
    /// external transport (the bundled signaling server, copy-paste, QR, etc.).
    Bootstrap,

    /// Newcomer joining an existing mesh — signaling is relayed over the data
    /// channel of the given intermediary peer; no external transport needed.
    Relay(PeerID),
}
