use crate::{
    client_fsm::transport::TransportFSM,
    state::{TransportInput, TransportOutput, TransportState},
};

/// Joiner-side transport FSM
pub struct Joiner {
    state: TransportState,
}

impl TransportFSM for Joiner {
    fn new() -> Self {
        Self {
            state: TransportState::Idle,
        }
    }

    fn state(&self) -> &TransportState {
        &self.state
    }

    fn process(&mut self, input: TransportInput) -> Option<TransportOutput> {
        match (&self.state, input) {
            (TransportState::Idle, TransportInput::SDPOfferReceived { sdp }) => {
                self.state = TransportState::CreatingAnswer;
                Some(TransportOutput::InitSDPAnswer { offer_sdp: sdp })
            }
            (TransportState::CreatingAnswer, TransportInput::SDPAnswerCreated { sdp }) => {
                self.state = TransportState::WaitingForDataChannel {
                    local_sdp: Some(sdp),
                };
                None
            }
            (TransportState::WaitingForDataChannel { .. }, TransportInput::DataChannelOpen) => {
                self.state = TransportState::Connected;
                None
            }
            (_, TransportInput::Disconnected) => {
                self.state = TransportState::Closed;
                Some(TransportOutput::Close)
            }
            _ => None,
        }
    }
}
