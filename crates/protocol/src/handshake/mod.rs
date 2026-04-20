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

    pub fn stragegy(&self) -> &HandshakeStrategy {
        &self.strategy
    }

    /// Process handshake input and generate handshake output
    pub fn process(&mut self, input: HandshakeInput) -> Result<Option<HandshakeOutput>> {
        self.fsm.process(input)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum HandshakeMode {
    /// Initial handshake with completely new peer, establishing first datachannel with that peer to our mesh
    Bootstrap,

    /// Handshake of newly connected peer with all others through "inviter" data channel.
    Relay(PeerID),
}
