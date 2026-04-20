use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState};
use anyhow::{Result, anyhow};

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

    pub fn process(&mut self, input: HandshakeInput) -> Result<Option<HandshakeOutput>> {
        match (&self.state, input) {
            (HandshakeState::Idle, HandshakeInput::Init) => {
                self.state = HandshakeState::CreatingOffer;
                Ok(Some(HandshakeOutput::InitSDPOffer))
            }
            (HandshakeState::CreatingOffer, HandshakeInput::OfferCreated(_)) => {
                self.state = HandshakeState::WaitingForAnswer;
                Ok(None)
            }
            (HandshakeState::WaitingForAnswer, HandshakeInput::Answer(answer)) => {
                self.state = HandshakeState::WaitingForDataChannel;
                Ok(Some(HandshakeOutput::AcceptSDPAnswer(answer)))
            }
            (HandshakeState::WaitingForDataChannel, HandshakeInput::DataChannelOpen) => {
                self.state = HandshakeState::Connected;
                Ok(Some(HandshakeOutput::Connected))
            }
            (_, HandshakeInput::Disconnected) => {
                self.state = HandshakeState::Closed;
                Ok(Some(HandshakeOutput::Close))
            }
            _ => Err(anyhow!("Unhandled input by host fsm")),
        }
    }
}
