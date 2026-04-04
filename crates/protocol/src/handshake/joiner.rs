use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState};

/// Joiner-side handshake FSM
pub struct Joiner {
    state: HandshakeState,
}

impl Joiner {
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
            (HandshakeState::Idle, HandshakeInput::SDPOfferReceived { sdp }) => {
                self.state = HandshakeState::CreatingAnswer;
                Some(HandshakeOutput::RequestSDPAnswer { offer: sdp })
            }
            (HandshakeState::CreatingAnswer, HandshakeInput::SDPAnswerCreated { .. }) => {
                self.state = HandshakeState::WaitingForDataChannel;
                None
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
