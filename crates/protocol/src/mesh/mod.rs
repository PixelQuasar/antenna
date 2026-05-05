use crate::{
    HandshakeFSM, HandshakeInput, HandshakeMode, HandshakeState, HandshakeStrategy, Identity,
    Input, MAX_RECONNECT_ATTEMPTS, MsgPayload, Output, PeerID, RECONNECT_INTERVAL_MS, RelayPayload,
    Scheduled, SignalingPayload, UserMsgPayload,
};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Default, Clone)]
struct DroppedPeerState {
    attempts: u32,
}

pub struct HandshakeContext {
    pub fsm: HandshakeFSM,
    pub mode: HandshakeMode,
}

/// Coarse mesh-membership state for the local node.
///
/// Recomputed after every [`MeshNodeFSM::process`] call. Once `Left`, stays `Left`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FSMState {
    /// No peer is `Connected` yet — the node has not joined any mesh.
    Init,
    /// At least one peer is `Connected`, but some relay handshake is still in progress.
    Connected,
    /// All peer handshakes have settled — the node is fully meshed and ready to send.
    Available,
    /// The node has issued [`Input::Leave`]; terminal.
    Left,
}

/// Top-level state machine for a mesh node.
///
/// Owns the local [`Identity`], every per-peer [`HandshakeFSM`], and the
/// coarse [`FSMState`]. [`MeshNodeFSM::process`] is the single entry point:
/// feed it an [`Input`], get back a `Vec<Output>`. No I/O happens here —
/// outputs are commands the driver must execute.
pub struct MeshNodeFSM {
    /// current peer ID
    id: PeerID,

    /// identity of current peer: id and key pair
    identity: Identity,

    /// Map of handshake automati, contains state of current handshakes with other sessions
    connections: HashMap<PeerID, HandshakeContext>,

    /// Pool of open-offer handshakes before the joiner's peer ID is known
    pending_handshakes: VecDeque<HandshakeContext>,

    /// Peers we lost abruptly and are trying to reconnect to. Cleared on successful
    /// reconnect (`HandshakeState::Connected`) or on `Input::Leave`.
    lost_peers: HashMap<PeerID, DroppedPeerState>,

    /// Coarse mesh-membership state; recomputed after each `process` call.
    /// Once `Left`, stays `Left`.
    state: FSMState,
}

impl Default for MeshNodeFSM {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshNodeFSM {
    pub fn new() -> Self {
        Self::with_identity(Identity::new())
    }

    pub fn with_identity(identity: Identity) -> Self {
        Self {
            id: identity.peer_id(),
            identity,
            connections: HashMap::new(),
            pending_handshakes: VecDeque::new(),
            lost_peers: HashMap::new(),
            state: FSMState::Init,
        }
    }

    pub fn state(&self) -> FSMState {
        self.state
    }

    fn compute_state(&self) -> FSMState {
        if self.state == FSMState::Left {
            return FSMState::Left;
        }
        if self.connected_peers().is_empty() {
            return FSMState::Init;
        }
        let in_progress_relays = self.connections.values().any(|ctx| {
            matches!(ctx.mode, HandshakeMode::Relay(_))
                && *ctx.fsm.state() != HandshakeState::Connected
        });
        if in_progress_relays {
            FSMState::Connected
        } else {
            FSMState::Available
        }
    }

    /// Outputs emitted for a state edge. Multi-step jumps (e.g. Init →
    fn state_transition_outputs<Msg: UserMsgPayload>(
        prev: FSMState,
        new: FSMState,
    ) -> Vec<Output<Msg>> {
        match (prev, new) {
            (FSMState::Init, FSMState::Connected) => vec![Output::Connected],
            (FSMState::Init, FSMState::Available) => {
                vec![Output::Connected, Output::Available]
            }
            (FSMState::Connected, FSMState::Available) => vec![Output::Available],
            (FSMState::Available, FSMState::Connected) => vec![Output::Unavailable],
            (FSMState::Connected | FSMState::Available, FSMState::Init) => {
                vec![Output::Unavailable]
            }
            (_, FSMState::Left) => vec![Output::Disconnecting],
            _ => vec![],
        }
    }

    pub fn id(&self) -> &PeerID {
        &self.id
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.connections.contains_key(peer)
            && *self.connections.get(peer).unwrap().fsm.state() == HandshakeState::Connected
    }

    /// Returns true if `peer` is allowed to deliver `msg` to us right now.
    pub fn channel_open_for_msg<Msg: UserMsgPayload>(
        &self,
        peer: &PeerID,
        msg: &MsgPayload<Msg>,
    ) -> bool {
        match msg {
            MsgPayload::RelaySignalingTo { .. } | MsgPayload::RelaySignalingFrom { .. } => {
                matches!(
                    self.connections.get(peer).map(|c| c.fsm.state()),
                    Some(HandshakeState::Connected | HandshakeState::WaitingForDataChannel)
                )
            }
            MsgPayload::User(_) | MsgPayload::Disconnect => self.is_connected(peer),
        }
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
        Ok(vec![])
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
        let offer = SignalingPayload {
            token: self.identity.create_token(&sdp)?,
            pubkey: self.identity.pubkey(),
        };
        self.pending_handshakes
            .back_mut()
            .ok_or_else(|| anyhow!("No pending open offer"))?
            .fsm
            .process(HandshakeInput::OfferCreated(sdp))?;
        Ok(vec![Output::OfferReady(offer)])
    }

    pub fn handle_send<Msg: UserMsgPayload>(
        &mut self,
        peer_to: PeerID,
        data: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        if self.is_connected(&peer_to) {
            Ok(vec![Output::SendMessage { peer_to, data }])
        } else {
            Ok(vec![])
        }
    }

    pub fn handle_broadcast<Msg: UserMsgPayload>(
        &mut self,

        data: MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        let mut out = vec![];
        for peer in self.connections.keys() {
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
        let prev_state = self.state;
        let mut outputs = self.dispatch(input)?;
        let new_state = self.compute_state();
        if prev_state != new_state {
            self.state = new_state;
            outputs.extend(Self::state_transition_outputs::<Msg>(prev_state, new_state));
        }
        Ok(outputs)
    }

    fn dispatch<Msg: UserMsgPayload>(&mut self, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
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
            Input::Leave => self.handle_leave(),
            Input::TimerFired { kind } => self.handle_timer_fired(kind),
        }
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Debug: snapshot of every connection's `(peer, state, mode)`.
    pub fn connections_snapshot(&self) -> Vec<(PeerID, HandshakeState, HandshakeMode)> {
        self.connections
            .iter()
            .map(|(peer, ctx)| (peer.clone(), ctx.fsm.state().clone(), ctx.mode.clone()))
            .collect()
    }

    /// Debug: number of contexts in `pending_handshakes`.
    pub fn pending_handshakes_len(&self) -> usize {
        self.pending_handshakes.len()
    }

    fn handle_leave<Msg: UserMsgPayload>(&mut self) -> Result<Vec<Output<Msg>>> {
        if self.state == FSMState::Left {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for peer in self.connections.keys() {
            if self.is_connected(peer) {
                out.push(Output::SendMessage {
                    peer_to: peer.clone(),
                    data: MsgPayload::Disconnect,
                });
            }
        }
        self.connections.clear();
        self.pending_handshakes.clear();
        self.lost_peers.clear();
        self.state = FSMState::Left;
        Ok(out)
    }

    fn handle_peer_leaving<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        let was_connected = self.connections.remove(&peer);

        let mut out = Vec::new();
        if was_connected.is_some() {
            out.push(Output::PeerDisconnected { peer });
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
            match &event {
                HandshakeInput::Answer(_) => {
                    let ctx = self
                        .pending_handshakes
                        .pop_front()
                        .ok_or_else(|| anyhow!("Pending handshake not found"))?;
                    self.connections.insert(peer.clone(), ctx);
                }
                HandshakeInput::ConnectionDropped => {
                    return Ok(outputs);
                }
                _ => return Err(anyhow!("Handshake instance with peer not found")),
            }
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
                    self.lost_peers.remove(&peer);
                    outputs.push(Output::PeerConnected { peer: peer.clone() });
                    for existing in self.connections.keys() {
                        if !self.is_connected(existing) || *existing == peer {
                            continue;
                        }
                        outputs.push(Output::SendMessage {
                            peer_to: existing.clone(),
                            data: MsgPayload::RelaySignalingFrom {
                                src: peer.clone(),
                                data: RelayPayload::InitConnect(peer.clone()),
                            },
                        });
                        outputs.push(Output::SendMessage {
                            peer_to: peer.clone(),
                            data: MsgPayload::RelaySignalingFrom {
                                src: existing.clone(),
                                data: RelayPayload::InitConnect(existing.clone()),
                            },
                        });
                    }
                }
                HandshakeState::Closed => {
                    self.connections.remove(&peer);
                    self.lost_peers.entry(peer.clone()).or_default();
                    outputs.push(Output::PeerLost { peer: peer.clone() });
                    outputs.push(Output::ScheduleTimer {
                        kind: Scheduled::ReconnectAttempt { peer: peer.clone() },
                        after_ms: RECONNECT_INTERVAL_MS,
                    });

                    let orphans: Vec<PeerID> = self
                        .connections
                        .iter()
                        .filter(|(_, c)| {
                            matches!(&c.mode, HandshakeMode::Relay(via) if via == &peer)
                                && *c.fsm.state() != HandshakeState::Connected
                        })
                        .map(|(id, _)| id.clone())
                        .collect();
                    for orphan in orphans {
                        self.connections.remove(&orphan);
                        self.lost_peers.entry(orphan.clone()).or_default();
                        outputs.push(Output::ScheduleTimer {
                            kind: Scheduled::ReconnectAttempt { peer: orphan },
                            after_ms: RECONNECT_INTERVAL_MS,
                        });
                    }
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
        let ctx = self.connections.get(peer).unwrap();
        let mut outputs: Vec<Output<Msg>> = vec![];
        match &event {
            HandshakeInput::Offer(payload) | HandshakeInput::Answer(payload) => {
                payload.get_sdp_verified(peer)?;
            }
            HandshakeInput::AnswerCreated(answer) => {
                let answer = SignalingPayload {
                    token: self.identity.create_token(answer)?,
                    pubkey: self.identity.pubkey(),
                };
                match &ctx.mode {
                    HandshakeMode::Bootstrap => outputs.push(Output::AnswerReady(answer)),
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
                    token: self.identity.create_token(offer)?,
                    pubkey: self.identity.pubkey(),
                };
                match &ctx.mode {
                    HandshakeMode::Bootstrap => outputs.push(Output::OfferReady(offer)),
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
        if !self.channel_open_for_msg(&peer, &msg) {
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
            MsgPayload::Disconnect => self.handle_peer_leaving(peer),
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

    fn handle_timer_fired<Msg: UserMsgPayload>(
        &mut self,
        kind: Scheduled,
    ) -> Result<Vec<Output<Msg>>> {
        match kind {
            Scheduled::ReconnectAttempt { peer } => self.handle_reconnect_attempt(peer),
        }
    }

    fn handle_reconnect_attempt<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
    ) -> Result<Vec<Output<Msg>>> {
        if !self.lost_peers.contains_key(&peer) {
            return Ok(vec![]);
        }
        if self.is_connected(&peer) {
            self.lost_peers.remove(&peer);
            return Ok(vec![]);
        }
        if self.connections.contains_key(&peer) {
            return Ok(vec![]);
        }

        let attempts = {
            let state = self.lost_peers.get_mut(&peer).unwrap();
            state.attempts += 1;
            state.attempts
        };

        let mut outputs: Vec<Output<Msg>> = vec![];

        if attempts > MAX_RECONNECT_ATTEMPTS {
            self.lost_peers.remove(&peer);
            return Ok(outputs);
        }

        let relay_peer = match self.connected_peers().into_iter().min() {
            Some(peer) => peer,
            None => {
                self.lost_peers.remove(&peer);
                return Ok(outputs);
            }
        };

        let i_am_host = self.id < peer;
        outputs.push(Output::SendMessage {
            peer_to: relay_peer.clone(),
            data: MsgPayload::RelaySignalingTo {
                dst: peer.clone(),
                data: RelayPayload::InitConnect(self.id.clone()),
            },
        });

        if i_am_host {
            let init_outs = self.process::<Msg>(Input::InitHandshake {
                with: peer.clone(),
                mode: HandshakeMode::Relay(relay_peer),
                strategy: HandshakeStrategy::Host,
            })?;
            outputs.extend(init_outs);
            let step_outs = self.process::<Msg>(Input::Handshake {
                from: peer.clone(),
                event: HandshakeInput::Init,
            })?;
            outputs.extend(step_outs);
        }

        outputs.push(Output::ScheduleTimer {
            kind: Scheduled::ReconnectAttempt { peer },
            after_ms: RECONNECT_INTERVAL_MS,
        });

        Ok(outputs)
    }

    fn handle_relay_signaling_from<Msg: UserMsgPayload>(
        &mut self,
        via: PeerID,
        src: PeerID,
        data: RelayPayload,
    ) -> Result<Vec<Output<Msg>>> {
        match data {
            RelayPayload::InitConnect(_) => {
                if self.connections.contains_key(&src) {
                    return Ok(vec![]);
                }
                let strategy = if self.id < src {
                    HandshakeStrategy::Host
                } else {
                    HandshakeStrategy::Joiner
                };
                self.process::<Msg>(Input::InitHandshake {
                    with: src.clone(),
                    mode: HandshakeMode::Relay(via),
                    strategy: strategy.clone(),
                })?;
                match strategy {
                    HandshakeStrategy::Host => self.process::<Msg>(Input::Handshake {
                        from: src,
                        event: HandshakeInput::Init,
                    }),
                    HandshakeStrategy::Joiner => Ok(vec![]),
                }
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
