mod host;
mod joiner;
mod test;

pub use crate::client_fsm::transport::host::Host;
pub use crate::client_fsm::transport::joiner::Joiner;
use crate::state::{TransportInput, TransportOutput, TransportState};

/// Trait that is implemented by transport-side module of client FSM
pub trait TransportFSM {
    /// Create new transport
    fn new() -> Self;

    /// Get current transport state
    fn state(&self) -> &TransportState;

    /// Process transport input and generate transport output
    fn process(&mut self, input: TransportInput) -> Option<TransportOutput>;
}
