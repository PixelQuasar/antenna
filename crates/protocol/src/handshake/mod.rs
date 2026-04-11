mod host;
mod input;
mod joiner;
mod output;
mod state;
mod strategy;
mod test;

use anyhow::Result;
pub use host::Host;
pub use input::HandshakeInput;
pub use joiner::Joiner;
pub use output::HandshakeOutput;
use serde::{Deserialize, Serialize};
pub use state::HandshakeState;
pub use strategy::{HandshakeStrategy, StrategyFSM};

use crate::PeerID;

pub struct HandshakeFSM {
    strategy: HandshakeStrategy,
    mode: HandshakeMode,
    fsm: StrategyFSM,
}

impl HandshakeFSM {
    pub fn new(mode: HandshakeMode, strategy: HandshakeStrategy) -> Self {
        match strategy {
            HandshakeStrategy::Host => Self::host(mode),
            HandshakeStrategy::Joiner => Self::joiner(mode),
        }
    }

    pub fn host(mode: HandshakeMode) -> Self {
        Self {
            strategy: HandshakeStrategy::Host,
            fsm: StrategyFSM::Host(Host::new()),
            mode,
        }
    }

    pub fn joiner(mode: HandshakeMode) -> Self {
        Self {
            strategy: HandshakeStrategy::Joiner,
            mode,
            fsm: StrategyFSM::Joiner(Joiner::new()),
        }
    }

    /// Get current handshake stat
    pub fn state(&self) -> &HandshakeState {
        self.fsm.state()
    }

    pub fn mode(&self) -> &HandshakeMode {
        &self.mode
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
