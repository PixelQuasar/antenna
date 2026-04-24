#[cfg(test)]
mod test;

use crate::{
    HandshakeFSM, HandshakeInput, HandshakeMode, HandshakeState, HandshakeStrategy, Identity,
    Input, MsgPayload, Output, PeerID, RelayPayload, SignalingPayload, UserMsgPayload,
};
use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use std::collections::{HashMap, HashSet, VecDeque};

///
#[derive(Default, Clone)]
pub struct MeshMetadata {
    ///
    pub offer: Option<SignalingPayload>,

    ///
    pub answer: Option<SignalingPayload>,
}

pub struct HandshakeContext {
    pub fsm: HandshakeFSM,
    pub mode: HandshakeMode,
}

/// Core FSM of antenna client, handles negotiation handshakes (but not signaling!!)
/// and abstract mesh logic
pub struct MeshNodeFSM {
    /// current peer ID
    id: PeerID,

    /// identity of current peer: id and key pair
    identity: Identity,

    /// Map of handshake automati, contains state of current handshakes with other sessions
    connections: HashMap<PeerID, HandshakeContext>,

    /// Pool of open-offer handshakes before the joiner's peer ID is known
    pending_handshakes: VecDeque<HandshakeContext>,

    ///
    metadata: MeshMetadata,

    /// True once Output::Available has been emitted (one-shot)
    available: bool,
}

impl MeshNodeFSM {
    pub fn new() -> Self {
        Self::with_identity(Identity::new())
    }

    pub fn with_identity(identity: Identity) -> Self {
        Self {
            id: PeerID::new(BASE64_URL_SAFE_NO_PAD.encode(identity.pubkey().to_bytes())),
            identity,
            connections: HashMap::new(),
            pending_handshakes: VecDeque::new(),
            metadata: MeshMetadata::default(),
            available: false,
        }
    }

    pub fn id(&self) -> &PeerID {
        &self.id
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.connections.contains_key(peer)
            && *self.connections.get(peer).unwrap().fsm.state() == HandshakeState::Connected
    }

    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.connections
            .iter()
            .filter(|x| *x.1.fsm.state() == HandshakeState::Connected)
            .map(|x| x.0.clone())
            .collect()
    }

    pub fn connected_number(&self) -> usize {
        self.connections.iter().fold(0, |a, x| {
            if *x.1.fsm.state() == HandshakeState::Connected {
                a + 1
            } else {
                a
            }
        })
    }

    pub fn handle_init_handshake<Msg: UserMsgPayload>(
        &mut self,
        with: PeerID,
        mode: HandshakeMode,
        strategy: HandshakeStrategy,
    ) -> Result<Vec<Output<Msg>>> {
        self.connections.insert(
            with,
            HandshakeContext {
                fsm: HandshakeFSM::new(strategy),
                mode,
            },
        );
        return Ok(vec![]);
    }

    pub fn handle_init_open_offer<Msg: UserMsgPayload>(&mut self) -> Result<Vec<Output<Msg>>> {
        let mut ctx = HandshakeContext {
            fsm: HandshakeFSM::new(HandshakeStrategy::Host),
            mode: HandshakeMode::Bootstrap,
        };
        ctx.fsm.process(HandshakeInput::Init)?;
        self.pending_handshakes.push_back(ctx);
        Ok(vec![Output::InitOpenOffer])
    }

    pub fn handle_open_offer_created<Msg: UserMsgPayload>(
        &mut self,
        sdp: String,
    ) -> Result<Vec<Output<Msg>>> {
        self.metadata.offer = Some(SignalingPayload {
            token: self.identity.create_token(&sdp)?,
            sdp: sdp.clone(),
            pubkey: self.identity.pubkey(),
        });
        self.pending_handshakes
            .back_mut()
            .ok_or_else(|| anyhow!("No pending open offer"))?
            .fsm
            .process(HandshakeInput::OfferCreated(sdp))?;
        Ok(vec![])
    }

    pub fn handle_send<Msg: UserMsgPayload>(
        &mut self,
        peer_to: PeerID,
        data: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        if self.is_connected(&peer_to) {
            Ok(vec![Output::SendMessage {
                peer_to,
                data: data,
            }])
        } else {
            Ok(vec![])
        }
    }

    pub fn handle_broadcast<Msg: UserMsgPayload>(
        &mut self,

        data: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        let mut out = vec![];
        for (peer, _) in &self.connections {
            if !self.is_connected(peer) {
                continue;
            }
            out.push(Output::SendMessage {
                peer_to: peer.clone(),
                data: data.clone(),
            })
        }
        Ok(out)
    }

    pub fn process<Msg: UserMsgPayload>(&mut self, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        match input {
            Input::InitHandshake {
                with,
                mode,
                strategy,
            } => self.handle_init_handshake(with, mode, strategy),
            Input::InitOpenOffer => self.handle_init_open_offer(),
            Input::OpenOfferCreated(sdp) => self.handle_open_offer_created(sdp),
            Input::Handshake { from, event } => self.handle_handshake(from, event),
            Input::PeerLeaving { peer } => self.handle_peer_leaving(peer),
            Input::MessageReceived { peer_from, data } => self.handle_message(peer_from, data),
            Input::Send { peer_to, data } => self.handle_send(peer_to, data),
            Input::Broadcast { data } => self.handle_broadcast(data),
        }
    }

    pub fn metadata(&self) -> &MeshMetadata {
        &self.metadata
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    fn handle_peer_leaving<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        let was_connected = self.connections.remove(&peer);

        let mut out = Vec::new();
        if was_connected.is_some() {
            out.push(Output::PeerDisconnected { peer });
            if self.available
                && self
                    .connections
                    .values()
                    .any(|ctx| *ctx.fsm.state() != HandshakeState::Connected)
            {
                self.available = false;
                out.push(Output::Unavailable);
            }
        }
        Ok(out)
    }

    pub(crate) fn handle_handshake<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
        event: HandshakeInput,
    ) -> Result<Vec<Output<Msg>>> {
        let mut outputs: Vec<Output<Msg>> = vec![];

        if !self.connections.contains_key(&peer) {
            let HandshakeInput::Answer(_) = &event else {
                return Err(anyhow!("Handshake instance with peer not found"));
            };
            let ctx = self
                .pending_handshakes
                .pop_front()
                .ok_or_else(|| anyhow!("Pending handshake not found"))?;
            self.connections.insert(peer.clone(), ctx);
            outputs.push(Output::InitOpenOffer);
        }

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
                    self.identity.add_known_peer(peer.clone());
                    outputs.push(Output::PeerConnected { peer: peer.clone() });
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

        if !self.available {
            let in_progress_relays = self
                .connections
                .values()
                .filter(|ctx| {
                    matches!(ctx.mode, HandshakeMode::Relay(_))
                        && *ctx.fsm.state() != HandshakeState::Connected
                })
                .count();
            if in_progress_relays == 0 && !self.connected_peers().is_empty() {
                self.available = true;
                outputs.push(Output::Available);
            }
        }

        Ok(outputs)
    }

    fn handle_side_effects<Msg: UserMsgPayload>(
        &mut self,
        peer: &PeerID,
        event: &HandshakeInput,
    ) -> Result<Vec<Output<Msg>>> {
        let ctx = self.connections.get(peer).unwrap();
        let mut outputs: Vec<Output<Msg>> = vec![];
        match &event {
            HandshakeInput::Offer(payload) | HandshakeInput::Answer(payload) => {
                self.identity.verify(payload, &peer)?;
            }
            HandshakeInput::AnswerCreated(answer) => {
                let answer = SignalingPayload {
                    token: self.identity.create_token(&answer)?,
                    sdp: answer.clone(),
                    pubkey: self.identity.pubkey(),
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
                    token: self.identity.create_token(&offer)?,
                    sdp: offer.clone(),
                    pubkey: self.identity.pubkey(),
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
                let mut out = vec![];
                if self.available {
                    self.available = false;
                    out.push(Output::Unavailable);
                }
                self.process::<Msg>(Input::InitHandshake {
                    with: src,
                    mode: HandshakeMode::Relay(via),
                    strategy: HandshakeStrategy::Joiner,
                })?;
                Ok(out)
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
