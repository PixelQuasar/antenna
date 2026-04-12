use crate::SignalingPayload;
use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState};
use anyhow::{Result, anyhow};

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

    pub fn process(&mut self, input: HandshakeInput) -> Result<Option<HandshakeOutput>> {
        match (&self.state, input) {
            (HandshakeState::Idle, HandshakeInput::Signaling(SignalingPayload::Offer(offer))) => {
                self.state = HandshakeState::CreatingAnswer;
                Ok(Some(HandshakeOutput::RequestSDPAnswer { offer }))
            }
            (
                HandshakeState::CreatingAnswer,
                HandshakeInput::SignalingCreated(SignalingPayload::Answer(_)),
            ) => {
                self.state = HandshakeState::WaitingForDataChannel;
                Ok(None)
            }
            (HandshakeState::WaitingForDataChannel, HandshakeInput::DataChannelOpen) => {
                self.state = HandshakeState::Connected;
                Ok(None)
            }
            (_, HandshakeInput::Disconnected) => {
                self.state = HandshakeState::Closed;
                Ok(Some(HandshakeOutput::Close))
            }
            _ => Err(anyhow!("Unhandled input by joiner fsm")),
        }
    }
}
