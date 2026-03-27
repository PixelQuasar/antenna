mod test;

use crate::state::{Input, Output};
pub use crate::transport::{TransportFSM, TransportState};

pub struct MeshFSM<T: TransportFSM> {
    transport: T,
}

impl<T: TransportFSM> MeshFSM<T> {
    pub fn new() -> Self {
        Self {
            transport: T::new(),
        }
    }

    pub fn state(&self) -> &TransportState {
        self.transport.state()
    }

    pub fn is_connected(&self) -> bool {
        *self.state() == TransportState::Connected
    }

    pub fn local_sdp(&self) -> Option<String> {
        match &self.state() {
            TransportState::WaitingForAnswer { local_sdp } => Some(local_sdp.to_string()),
            TransportState::WaitingForDataChannel { local_sdp } => local_sdp.clone(),
            _ => None,
        }
    }

    pub fn process<Msg>(&mut self, input: Input<Msg>) -> Option<Output<Msg>> {
        match input {
            Input::Transport(event) => self.transport.process(event).map(Output::Transport),
            Input::MessageReceived { peer_from, data } => {
                if self.is_connected() {
                    Some(Output::ReceiveMessage { peer_from, data })
                } else {
                    None
                }
            }
            Input::PeerSend { peer_to, data } => {
                if self.is_connected() {
                    Some(Output::SendMessage { peer_to, data })
                } else {
                    None
                }
            }
            Input::PeerBroadcast { data } => {
                if self.is_connected() {
                    Some(Output::Broadcast { data })
                } else {
                    None
                }
            }
        }
    }
}
