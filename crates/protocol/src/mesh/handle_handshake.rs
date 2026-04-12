use anyhow::{Result, anyhow};

use crate::{
    HandshakeInput, HandshakeMode, HandshakeState, MeshNodeFSM, MsgPayload, Output, PeerID,
    RelayPayload, SignalingPayload, UserMsgPayload,
};

impl MeshNodeFSM {
    pub(crate) fn handle_handshake<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
        event: HandshakeInput,
    ) -> Result<Vec<Output<Msg>>> {
        if !self.handshakes.contains_key(&peer) {
            return Err(anyhow!("Handshake instance with peer not found"));
        }

        let ctx = self.handshakes.get_mut(&peer).unwrap();
        let handshake_out = ctx.fsm.process(event.clone())?;
        let state = ctx.fsm.state();
        let mode = ctx.mode.clone();

        let mut outputs: Vec<Output<Msg>> = vec![];

        if let Some(event) = handshake_out {
            outputs.push(Output::Handshake {
                peer: peer.clone(),
                event,
            });
        }

        match &event {
            HandshakeInput::SignalingCreated(payload) => match &mode {
                HandshakeMode::Bootstrap => match payload {
                    SignalingPayload::Offer(sdp) => {
                        self.metadata.sdp_offer = Some(sdp.clone());
                    }
                    SignalingPayload::Answer(sdp) => {
                        self.metadata.sdp_answer = Some(sdp.clone());
                    }
                },
                HandshakeMode::Relay(via) => {
                    outputs.push(Output::SendMessage {
                        peer_to: via.clone(),
                        data: MsgPayload::RelaySignalingTo {
                            dst: peer.clone(),
                            data: RelayPayload::Signaling(payload.clone()),
                        },
                    });
                }
            },
            _ => {}
        }

        match state {
            HandshakeState::Connected => {
                self.handshakes.remove(&peer);
                self.connected.insert(peer.clone());

                for existing in &self.connected {
                    if existing != &peer {
                        outputs.push(Output::PeerAppeared {
                            peer: existing.clone(),
                        });

                        if mode == HandshakeMode::Bootstrap {
                            outputs.push(Output::SendMessage {
                                peer_to: existing.clone(),
                                data: MsgPayload::RelaySignalingFrom {
                                    src: peer.clone(),
                                    data: RelayPayload::InitHost,
                                },
                            });
                            outputs.push(Output::SendMessage {
                                peer_to: peer.clone(),
                                data: MsgPayload::RelaySignalingFrom {
                                    src: existing.clone(),
                                    data: RelayPayload::InitJoiner,
                                },
                            });
                        }
                    }
                }
            }
            HandshakeState::Closed => {
                self.handshakes.remove(&peer);
                outputs.push(Output::PeerDisconnected { peer: peer.clone() });
            }
            _ => {}
        }

        Ok(outputs)
    }
}
