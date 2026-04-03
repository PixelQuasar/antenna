mod host;
mod input;
mod joiner;
mod output;
mod state;
mod test;

pub use host::Host;
pub use input::HandshakeInput;
pub use joiner::Joiner;
pub use output::HandshakeOutput;
pub use state::HandshakeState;

use crate::mesh::PeerID;

/// Trait that is implemented by handshake-side module of client FSM
pub enum HandshakeFSM {
    Host(Host),
    Joiner(Joiner),
}

impl HandshakeFSM {
    /// Get current handshake stat
    pub fn state(&self) -> &HandshakeState {
        match self {
            Self::Host(host) => host.state(),
            Self::Joiner(joiner) => joiner.state(),
        }
    }

    /// Process handshake input and generate handshake output
    pub fn process(&mut self, input: HandshakeInput) -> Option<HandshakeOutput> {
        match self {
            Self::Host(host) => host.process(input),
            Self::Joiner(joiner) => joiner.process(input),
        }
    }
}

pub struct HandshakeContext {
    /// Peer ID from through we are handshaking. If none, handshake is direct
    pub via: Option<PeerID>,

    /// Handshake state machine
    pub handshake: HandshakeFSM,
}
