use antenna_protocol::UserMsgPayload;

use crate::Peer;

/// DRAFT
pub struct SignalingClient<'a, Msg: UserMsgPayload + 'static> {
    peer: &'a mut Peer<Msg>,
}

impl<'a, Msg: UserMsgPayload> SignalingClient<'a, Msg> {
    pub fn new(peer: &'a mut Peer<Msg>) -> Self {
        Self { peer }
    }

    pub fn join(&mut self, id: String) {
        todo!()
    }

    fn smth(&mut self) {
        self.peer.start();
    }
}
