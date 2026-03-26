mod host;
mod joiner;

pub use crate::client_fsm::transport::host::Host;
pub use crate::client_fsm::transport::joiner::Joiner;
use crate::state::{TransportInput, TransportOutput, TransportState};

pub trait TransportFSM {
    fn new() -> Self;

    fn state(&self) -> &TransportState;

    fn process(&mut self, input: TransportInput) -> Option<TransportOutput>;
}
