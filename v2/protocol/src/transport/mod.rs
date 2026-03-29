mod host;
mod input;
mod joiner;
mod output;
mod state;
mod test;

pub use host::Host;
pub use input::TransportInput;
pub use joiner::Joiner;
pub use output::TransportOutput;
pub use state::TransportState;

/// Trait that is implemented by transport-side module of client FSM
pub enum TransportFSM {
    Host(Host),
    Joiner(Joiner),
}

impl TransportFSM {
    /// Get current transport stat
    pub fn state(&self) -> &TransportState {
        match self {
            Self::Host(host) => host.state(),
            Self::Joiner(joiner) => joiner.state(),
        }
    }

    /// Process transport input and generate transport output
    pub fn process(&mut self, input: TransportInput) -> Option<TransportOutput> {
        match self {
            Self::Host(host) => host.process(input),
            Self::Joiner(joiner) => joiner.process(input),
        }
    }
}
