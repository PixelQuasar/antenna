use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState};

/// Host-side handshake FSM
pub struct Host {
    state: HandshakeState,
}

impl Host {
    pub fn new() -> Self {
        Self {
            state: HandshakeState::Idle,
        }
    }

    pub fn state(&self) -> &HandshakeState {
        &self.state
    }

    pub fn process(&mut self, input: HandshakeInput) -> Option<HandshakeOutput> {
        match (&self.state, input) {
            (HandshakeState::Idle, HandshakeInput::InitNegotiation { .. }) => {
                self.state = HandshakeState::CreatingOffer;
                Some(HandshakeOutput::InitSDPOffer)
            }
            (HandshakeState::CreatingOffer, HandshakeInput::SDPOfferCreated { .. }) => {
                self.state = HandshakeState::WaitingForAnswer;
                None
            }
            (HandshakeState::WaitingForAnswer, HandshakeInput::SDPAnswerReceived { sdp }) => {
                self.state = HandshakeState::WaitingForDataChannel;
                Some(HandshakeOutput::AcceptSDPAnswer { answer: sdp })
            }
            (HandshakeState::WaitingForDataChannel, HandshakeInput::DataChannelOpen) => {
                self.state = HandshakeState::Connected;
                None
            }
            (_, HandshakeInput::Disconnected) => {
                self.state = HandshakeState::Closed;
                Some(HandshakeOutput::Close)
            }
            _ => None,
        }
    }
}
