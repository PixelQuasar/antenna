use crate::{
    HandshakeInput, HandshakeMode, HandshakeStrategy, Input, MeshNodeFSM, MsgPayload, Output,
    PeerID, RelayPayload, UserMsgPayload,
};
use anyhow::Result;

impl MeshNodeFSM {
    pub(crate) fn handle_message<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
        msg: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        if !self.is_connected(&peer) {
            return Ok(vec![]);
        }

        match msg {
            MsgPayload::RelaySignalingTo { dst, data } => {
                self.handle_relay_signaling_to(peer, dst, data)
            }
            MsgPayload::RelaySignalingFrom { src, data } => {
                self.handle_relay_signaling_from(peer, src, data)
            }
            MsgPayload::User(_) => Ok(vec![Output::ReceiveMessage {
                peer_from: peer,
                data: msg,
            }]),
            _ => Ok(vec![]),
        }
    }

    fn handle_relay_signaling_to<Msg: UserMsgPayload>(
        &mut self,
        src: PeerID,
        dst: PeerID,
        data: RelayPayload,
    ) -> Result<Vec<Output<Msg>>> {
        Ok(vec![Output::SendMessage {
            peer_to: dst,
            data: MsgPayload::RelaySignalingFrom { src, data },
        }])
    }

    fn handle_relay_signaling_from<Msg: UserMsgPayload>(
        &mut self,
        via: PeerID,
        src: PeerID,
        data: RelayPayload,
    ) -> Result<Vec<Output<Msg>>> {
        match data {
            RelayPayload::InitHost(_) => {
                if self.connections.contains_key(&src) {
                    return Ok(vec![]);
                }
                self.process::<Msg>(Input::InitHandshake {
                    with: src.clone(),
                    mode: HandshakeMode::Relay(via),
                    strategy: HandshakeStrategy::Host,
                })?;
                self.process::<Msg>(Input::Handshake {
                    from: src,
                    event: HandshakeInput::Init,
                })
            }
            RelayPayload::InitJoiner(_) => {
                if self.connections.contains_key(&src) {
                    return Ok(vec![]);
                }
                self.process::<Msg>(Input::InitHandshake {
                    with: src,
                    mode: HandshakeMode::Relay(via),
                    strategy: HandshakeStrategy::Joiner,
                })
            }
            RelayPayload::Offer(offer) => self.process::<Msg>(Input::Handshake {
                from: src,
                event: HandshakeInput::Offer(offer),
            }),
            RelayPayload::Answer(answer) => self.process::<Msg>(Input::Handshake {
                from: src,
                event: HandshakeInput::Answer(answer),
            }),
        }
    }
}
