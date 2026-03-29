use crate::transport::{TransportInput, TransportOutput, TransportState};

/// Host-side transport FSM
pub struct Host {
    state: TransportState,
}

impl Host {
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
            (TransportState::Idle, TransportInput::InitNegotiation) => {
                self.state = TransportState::CreatingOffer;
                Some(TransportOutput::InitSDPOffer)
            }
            (TransportState::CreatingOffer, TransportInput::SDPOfferCreated { sdp }) => {
                self.state = TransportState::WaitingForAnswer { local_sdp: sdp };
                None
            }
            (
                TransportState::WaitingForAnswer { .. },
                TransportInput::SDPAnswerReceived { sdp },
            ) => {
                self.state = TransportState::WaitingForDataChannel { local_sdp: None };
                Some(TransportOutput::AcceptSDPAnswer { sdp })
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
