mod host;
mod joiner;
mod test;

use crate::state::{TransportInput, TransportOutput, TransportState};
pub use host::Host;
pub use joiner::Joiner;

/// Trait that is implemented by transport-side module of client FSM
pub trait TransportFSM {
    /// Create new transport
    fn new() -> Self;

    /// Get current transport state
    fn state(&self) -> &TransportState;

    /// Process transport input and generate transport output
    fn process(&mut self, input: TransportInput) -> Option<TransportOutput>;
}
