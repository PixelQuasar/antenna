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
pub trait TransportFSM {
    /// Create new transport
    fn new() -> Self;

    /// Get current transport state
    fn state(&self) -> &TransportState;

    /// Process transport input and generate transport output
    fn process(&mut self, input: TransportInput) -> Option<TransportOutput>;
}
