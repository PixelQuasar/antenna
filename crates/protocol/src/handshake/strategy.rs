use serde::{Deserialize, Serialize};

use crate::{HandshakeInput, HandshakeOutput, HandshakeState, Host, Joiner};
use anyhow::Result;

/// Which side of the handshake the local peer plays.
///
/// In a relay handshake, the side with the smaller [`crate::PeerID`] becomes
/// `Host`, the larger becomes `Joiner` — both choose independently and
/// always agree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeStrategy {
    /// Sends the SDP offer first, then accepts the answer.
    Host,
    /// Receives the offer, returns an answer, awaits the data channel.
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
