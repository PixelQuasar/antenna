mod test;
mod transport;

pub use crate::client_fsm::transport::{Host, Joiner, TransportFSM};

use crate::state::{Input, Output, TransportState};

pub struct ClientFSM<T: TransportFSM> {
    transport: T,
}

impl<T: TransportFSM> ClientFSM<T> {
    pub fn new() -> Self {
        Self {
            transport: T::new(),
        }
    }

    pub fn state(&self) -> &TransportState {
        self.transport.state()
    }

    fn connected(&self) -> bool {
        *self.state() == TransportState::Connected
    }

    pub fn process<Msg>(&mut self, input: Input<Msg>) -> Option<Output<Msg>> {
        match input {
            Input::Transport(event) => self.transport.process(event).map(Output::Transport),
            Input::MessageReceived { peer_from, data } => {
                if self.connected() {
                    Some(Output::ReceiveMessage { peer_from, data })
                } else {
                    None
                }
            }
            Input::PeerSend { peer_to, data } => {
                if self.connected() {
                    Some(Output::SendMessage { peer_to, data })
                } else {
                    None
                }
            }
            Input::PeerBroadcast { data } => {
                if self.connected() {
                    Some(Output::Broadcast { data })
                } else {
                    None
                }
            }
        }
    }
}
