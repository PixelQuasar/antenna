use crate::{MeshNodeFSM, MsgPayload, Output, PeerID, UserMsgPayload};
use anyhow::Result;

impl MeshNodeFSM {
    pub(crate) fn handle_message<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
        msg: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        if !self.connected.contains(&peer) {
            return Ok(vec![]);
        }

        match msg {
            MsgPayload::PeerJoined(peer) => self.handle_peer_joined(peer),
            other => Ok(vec![Output::ReceiveMessage {
                peer_from: peer,
                data: other,
            }]),
        }
    }
}
