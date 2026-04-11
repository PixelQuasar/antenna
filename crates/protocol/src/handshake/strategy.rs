use serde::{Deserialize, Serialize};

use crate::{HandshakeInput, HandshakeOutput, HandshakeState, Host, Joiner};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeStrategy {
    Host,
    Joiner,
}

pub enum StrategyFSM {
    Host(Host),
    Joiner(Joiner),
}

impl StrategyFSM {
    pub fn state(&self) -> &HandshakeState {
        match self {
            Self::Host(host) => host.state(),
            Self::Joiner(joiner) => joiner.state(),
        }
    }

    pub fn process(&mut self, input: HandshakeInput) -> Result<Option<HandshakeOutput>> {
        match self {
            Self::Host(host) => host.process(input),
            Self::Joiner(joiner) => joiner.process(input),
        }
    }
}
