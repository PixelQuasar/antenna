use crate::transport::{TransportInput, TransportOutput, TransportState};

/// Joiner-side transport FSM
pub struct Joiner {
    state: TransportState,
}

impl Joiner {
    pub fn new() -> Self {
        Self {
            state: TransportState::Idle,
        }
    }

    pub fn state(&self) -> &TransportState {
        &self.state
    }

    pub fn process(&mut self, input: TransportInput) -> Option<TransportOutput> {
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
