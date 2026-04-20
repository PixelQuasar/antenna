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
        if !self.connections.contains_key(&peer) {
            return Err(anyhow!("Handshake instance with peer not found"));
        }

        let mut outputs: Vec<Output<Msg>> = vec![];

        let side_effects_outs = self.handle_side_effects(&peer, &event)?;
        outputs.extend(side_effects_outs);

        let handshake_out = {
            let ctx = self.connections.get_mut(&peer);
            if let Some(ctx) = ctx {
                ctx.fsm.process(event.clone())?
            } else {
                None
            }
        };

        if let Some(event) = handshake_out {
            outputs.push(Output::Handshake {
                peer: peer.clone(),
                event,
            });
        }

        let ctx = self.connections.get(&peer);
        if let Some(ctx) = ctx {
            match ctx.fsm.state() {
                HandshakeState::Connected => {
                    for (existing, _) in &self.connections {
                        if !self.is_connected(existing) || *existing == peer {
                            continue;
                        }

                        outputs.push(Output::PeerAppeared {
                            peer: existing.clone(),
                        });

                        if ctx.mode == HandshakeMode::Bootstrap {
                            outputs.push(Output::SendMessage {
                                peer_to: existing.clone(),
                                data: MsgPayload::RelaySignalingFrom {
                                    src: peer.clone(),
                                    data: RelayPayload::InitHost(peer.clone()),
                                },
                            });
                            outputs.push(Output::SendMessage {
                                peer_to: peer.clone(),
                                data: MsgPayload::RelaySignalingFrom {
                                    src: existing.clone(),
                                    data: RelayPayload::InitJoiner(existing.clone()),
                                },
                            });
                        }
                    }
                }
                HandshakeState::Closed => {
                    self.connections.remove(&peer);
                    outputs.push(Output::PeerDisconnected { peer: peer.clone() });
                }
                _ => {}
            }
        }

        Ok(outputs)
    }

    fn handle_side_effects<Msg: UserMsgPayload>(
        &mut self,
        peer: &PeerID,
        event: &HandshakeInput,
    ) -> Result<Vec<Output<Msg>>> {
        let ctx = self.connections.get(&peer).unwrap();
        let mut outputs: Vec<Output<Msg>> = vec![];
        match &event {
            HandshakeInput::Offer(payload) | HandshakeInput::Answer(payload) => {
                self.identity.verify(payload, &peer)?;
            }
            HandshakeInput::AnswerCreated(answer) => {
                let answer = SignalingPayload {
                    sdp: answer.clone(),
                    pubkey: self.identity.pubkey(),
                    token: self.identity.create_token(&peer)?.to_vec()?,
                };
                match &ctx.mode {
                    HandshakeMode::Bootstrap => self.metadata.answer = Some(answer),
                    HandshakeMode::Relay(via) => {
                        outputs.push(Output::SendMessage {
                            peer_to: via.clone(),
                            data: MsgPayload::RelaySignalingTo {
                                dst: peer.clone(),
                                data: RelayPayload::Answer(answer),
                            },
                        });
                    }
                }
            }
            HandshakeInput::OfferCreated(offer) => {
                let offer = SignalingPayload {
                    sdp: offer.clone(),
                    pubkey: self.identity.pubkey(),
                    token: self.identity.create_token(&peer)?.to_vec()?,
                };
                match &ctx.mode {
                    HandshakeMode::Bootstrap => self.metadata.offer = Some(offer),
                    HandshakeMode::Relay(via) => {
                        outputs.push(Output::SendMessage {
                            peer_to: via.clone(),
                            data: MsgPayload::RelaySignalingTo {
                                dst: peer.clone(),
                                data: RelayPayload::Offer(offer),
                            },
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(outputs)
    }
}
