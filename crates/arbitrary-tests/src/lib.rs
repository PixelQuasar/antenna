use antenna_protocol::{
    HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeStrategy, Identity, Input,
    MeshNodeFSM, MsgPayload, Output, PeerID, RelayPayload, Scheduled, SignalingPayload,
};
use proptest::prelude::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::OnceLock;

const IDENTITY_POOL_SIZE: usize = 6;

pub fn identity_pool() -> &'static [Identity] {
    static POOL: OnceLock<Vec<Identity>> = OnceLock::new();
    POOL.get_or_init(|| (0..IDENTITY_POOL_SIZE).map(|_| Identity::new()).collect())
}

pub fn identity_peer_ids() -> Vec<PeerID> {
    identity_pool()
        .iter()
        .map(|id| {
            SignalingPayload {
                pubkey: id.pubkey(),
                token: vec![],
            }
            .peer_id()
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum PayloadFlavor {
    Valid,
    TamperedToken,
    SwappedPubkey,
}

fn build_payload(i: usize, sdp: &str, flavor: PayloadFlavor) -> SignalingPayload {
    let pool = identity_pool();
    let id = &pool[i];
    let token = id.create_token(sdp).unwrap_or_default();
    match flavor {
        PayloadFlavor::Valid => SignalingPayload {
            pubkey: id.pubkey(),
            token,
        },
        PayloadFlavor::TamperedToken => {
            let mut t = token;
            if t.is_empty() {
                t.push(0x42);
            } else {
                t[0] ^= 0xAA;
            }
            SignalingPayload {
                pubkey: id.pubkey(),
                token: t,
            }
        }
        PayloadFlavor::SwappedPubkey => {
            let other = &pool[(i + 1) % pool.len()];
            SignalingPayload {
                pubkey: other.pubkey(),
                token,
            }
        }
    }
}

pub fn arb_payload_flavor() -> impl Strategy<Value = PayloadFlavor> {
    prop_oneof![
        4 => Just(PayloadFlavor::Valid),
        1 => Just(PayloadFlavor::TamperedToken),
        1 => Just(PayloadFlavor::SwappedPubkey),
    ]
}

pub fn arb_signaling_payload() -> impl Strategy<Value = SignalingPayload> {
    (
        0..IDENTITY_POOL_SIZE,
        ".{0,32}".prop_map(String::from),
        arb_payload_flavor(),
    )
        .prop_map(|(i, sdp, fl)| build_payload(i, &sdp, fl))
}

pub fn arb_matching_payload_pair() -> impl Strategy<Value = (PeerID, SignalingPayload)> {
    (
        0..IDENTITY_POOL_SIZE,
        ".{0,32}".prop_map(String::from),
        arb_payload_flavor(),
    )
        .prop_map(|(i, sdp, fl)| {
            let signer_pid = SignalingPayload {
                pubkey: identity_pool()[i].pubkey(),
                token: vec![],
            }
            .peer_id();
            (signer_pid, build_payload(i, &sdp, fl))
        })
}

/// Generates random `PeerID`s by sampling the long-lived `identity_pool()`.
/// Each PeerID is now a `PublicKey`, so we can't conjure one from a random
/// string — we draw from the pre-generated identity pool that's already
/// used for signed payloads.
pub fn arb_peer_id() -> impl Strategy<Value = PeerID> {
    proptest::sample::select(identity_peer_ids())
}

pub fn arb_peer_from_pool(pool: Vec<PeerID>) -> impl Strategy<Value = PeerID> {
    proptest::sample::select(pool)
}

pub fn arb_handshake_strategy() -> impl Strategy<Value = HandshakeStrategy> {
    prop_oneof![
        Just(HandshakeStrategy::Host),
        Just(HandshakeStrategy::Joiner)
    ]
}

pub fn arb_handshake_mode(pool: Vec<PeerID>) -> impl Strategy<Value = HandshakeMode> {
    prop_oneof![
        Just(HandshakeMode::Bootstrap),
        arb_peer_from_pool(pool).prop_map(HandshakeMode::Relay),
    ]
}

pub fn arb_handshake_input() -> impl Strategy<Value = HandshakeInput> {
    prop_oneof![
        1 => Just(HandshakeInput::Init),
        1 => ".{0,32}".prop_map(HandshakeInput::OfferCreated),
        1 => ".{0,32}".prop_map(HandshakeInput::AnswerCreated),
        1 => Just(HandshakeInput::DataChannelOpen),
        1 => Just(HandshakeInput::ConnectionDropped),
        2 => arb_signaling_payload().prop_map(HandshakeInput::Offer),
        2 => arb_signaling_payload().prop_map(HandshakeInput::Answer),
    ]
}

pub fn arb_relay_payload(pool: Vec<PeerID>) -> impl Strategy<Value = RelayPayload> {
    prop_oneof![
        1 => arb_peer_from_pool(pool).prop_map(RelayPayload::InitConnect),
        1 => arb_signaling_payload().prop_map(RelayPayload::Offer),
        1 => arb_signaling_payload().prop_map(RelayPayload::Answer),
    ]
}

pub fn arb_msg_payload(pool: Vec<PeerID>) -> impl Strategy<Value = MsgPayload<()>> {
    prop_oneof![
        Just(MsgPayload::User(())),
        Just(MsgPayload::Disconnect),
        (
            arb_peer_from_pool(pool.clone()),
            arb_relay_payload(pool.clone())
        )
            .prop_map(|(dst, data)| MsgPayload::RelaySignalingTo { dst, data }),
        (arb_peer_from_pool(pool.clone()), arb_relay_payload(pool))
            .prop_map(|(src, data)| MsgPayload::RelaySignalingFrom { src, data }),
    ]
}

pub fn arb_input(pool: Vec<PeerID>) -> impl Strategy<Value = Input<()>> {
    prop_oneof![
        (
            arb_peer_from_pool(pool.clone()),
            arb_handshake_mode(pool.clone()),
            arb_handshake_strategy()
        )
            .prop_map(|(with, mode, strategy)| Input::InitHandshake {
                with,
                mode,
                strategy
            }),
        Just(Input::InitOpenOffer),
        ".{0,32}".prop_map(Input::OpenOfferCreated),
        (arb_peer_from_pool(pool.clone()), arb_handshake_input())
            .prop_map(|(from, event)| Input::Handshake { from, event }),
        arb_matching_payload_pair().prop_map(|(from, payload)| Input::Handshake {
            from,
            event: HandshakeInput::Offer(payload),
        }),
        arb_matching_payload_pair().prop_map(|(from, payload)| Input::Handshake {
            from,
            event: HandshakeInput::Answer(payload),
        }),
        (
            arb_peer_from_pool(pool.clone()),
            arb_msg_payload(pool.clone())
        )
            .prop_map(|(peer_from, data)| Input::MessageReceived { peer_from, data }),
        (
            arb_peer_from_pool(pool.clone()),
            arb_msg_payload(pool.clone())
        )
            .prop_map(|(peer_to, data)| Input::Send { peer_to, data }),
        arb_msg_payload(pool.clone()).prop_map(|data| Input::Broadcast { data }),
        arb_peer_from_pool(pool.clone()).prop_map(|peer| Input::PeerLeaving { peer }),
        Just(Input::Leave),
        arb_peer_from_pool(pool).prop_map(|peer| Input::TimerFired {
            kind: Scheduled::ReconnectAttempt { peer }
        }),
    ]
}

/// A multi-peer mesh simulator over real `MeshNodeFSM`s
pub struct MeshSim {
    pub peers: BTreeMap<PeerID, MeshNodeFSM>,
    /// In-flight wire messages, partitioned per `(sender, receiver)` pair
    pub msg_queue: BTreeMap<(PeerID, PeerID), VecDeque<MsgPayload<()>>>,
    /// SDP-completion / data-channel-open inputs
    pub pending_inputs: BTreeMap<PeerID, VecDeque<Input<()>>>,
    /// Timers requested by the FSM, awaiting a `TimerFired` input.
    pub timers: BTreeMap<PeerID, VecDeque<Scheduled>>,
    /// Audit log of every observable FSM output we've seen.
    pub events: Vec<(PeerID, ObservedEvent)>,
}

#[derive(Debug, Clone)]
pub enum ObservedEvent {
    OfferReady(SignalingPayload),
    AnswerReady(SignalingPayload),
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
    PeerLost(PeerID),
    Available,
    Unavailable,
    Disconnecting,
    ReceiveMessage { peer_from: PeerID },
}

impl MeshSim {
    pub fn new(n: usize) -> Self {
        let mut peers = BTreeMap::new();
        for _ in 0..n {
            let fsm = MeshNodeFSM::new();
            peers.insert(fsm.id().clone(), fsm);
        }
        Self {
            peers,
            msg_queue: BTreeMap::new(),
            pending_inputs: BTreeMap::new(),
            timers: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    pub fn ids(&self) -> Vec<PeerID> {
        self.peers.keys().cloned().collect()
    }

    /// Process one input on a peer
    pub fn process(&mut self, peer: &PeerID, input: Input<()>) {
        let outs = match self.peers.get_mut(peer) {
            Some(fsm) => fsm.process::<()>(input).unwrap_or_default(),
            None => return,
        };
        for out in outs {
            self.handle_output(peer, out);
        }
    }

    fn handle_output(&mut self, peer: &PeerID, output: Output<()>) {
        match output {
            Output::SendMessage { peer_to, data } => {
                self.msg_queue
                    .entry((peer.clone(), peer_to))
                    .or_default()
                    .push_back(data);
            }
            Output::InitOpenOffer => {
                let sdp = format!("sdp-open-{peer}");
                self.pending_inputs
                    .entry(peer.clone())
                    .or_default()
                    .push_back(Input::OpenOfferCreated(sdp));
            }
            Output::Handshake {
                peer: cp,
                event: HandshakeOutput::InitSDPOffer,
            } => {
                let sdp = format!("sdp-offer-{peer}-{cp}");
                self.pending_inputs
                    .entry(peer.clone())
                    .or_default()
                    .push_back(Input::Handshake {
                        from: cp,
                        event: HandshakeInput::OfferCreated(sdp),
                    });
            }
            Output::Handshake {
                peer: cp,
                event: HandshakeOutput::RequestSDPAnswer(_),
            } => {
                let sdp = format!("sdp-answer-{peer}-{cp}");
                self.pending_inputs
                    .entry(peer.clone())
                    .or_default()
                    .push_back(Input::Handshake {
                        from: cp,
                        event: HandshakeInput::AnswerCreated(sdp),
                    });
            }
            Output::Handshake {
                peer: cp,
                event: HandshakeOutput::AcceptSDPAnswer(_),
            } => {
                self.pending_inputs
                    .entry(peer.clone())
                    .or_default()
                    .push_back(Input::Handshake {
                        from: cp.clone(),
                        event: HandshakeInput::DataChannelOpen,
                    });
                self.pending_inputs
                    .entry(cp.clone())
                    .or_default()
                    .push_back(Input::Handshake {
                        from: peer.clone(),
                        event: HandshakeInput::DataChannelOpen,
                    });
            }
            Output::Handshake { .. } => {}
            Output::OfferReady(payload) => {
                self.events
                    .push((peer.clone(), ObservedEvent::OfferReady(payload)));
            }
            Output::AnswerReady(payload) => {
                self.events
                    .push((peer.clone(), ObservedEvent::AnswerReady(payload)));
            }
            Output::PeerConnected { peer: cp } => self
                .events
                .push((peer.clone(), ObservedEvent::PeerConnected(cp))),
            Output::PeerDisconnected { peer: cp } => self
                .events
                .push((peer.clone(), ObservedEvent::PeerDisconnected(cp))),
            Output::PeerLost { peer: cp } => self
                .events
                .push((peer.clone(), ObservedEvent::PeerLost(cp))),
            Output::Available => self.events.push((peer.clone(), ObservedEvent::Available)),
            Output::Unavailable => self.events.push((peer.clone(), ObservedEvent::Unavailable)),
            Output::Disconnecting => self
                .events
                .push((peer.clone(), ObservedEvent::Disconnecting)),
            Output::ReceiveMessage { peer_from, .. } => self
                .events
                .push((peer.clone(), ObservedEvent::ReceiveMessage { peer_from })),
            Output::ScheduleTimer { kind, .. } => {
                self.timers.entry(peer.clone()).or_default().push_back(kind);
            }
        }
    }

    /// Pick the `idx % nonempty_pairs.len()`-th pair with pending messages
    /// and deliver its head-of-line message. Per-pair FIFO is preserved.
    pub fn deliver_msg_at(&mut self, idx: usize) -> bool {
        let pairs: Vec<(PeerID, PeerID)> = self
            .msg_queue
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, _)| k.clone())
            .collect();
        if pairs.is_empty() {
            return false;
        }
        let (from, to) = pairs[idx % pairs.len()].clone();
        let data = match self
            .msg_queue
            .get_mut(&(from.clone(), to.clone()))
            .and_then(|q| q.pop_front())
        {
            Some(d) => d,
            None => return false,
        };
        self.process(
            &to,
            Input::MessageReceived {
                peer_from: from,
                data,
            },
        );
        true
    }

    /// Flush exactly one queued SDP-completion / DC-open input for a peer.
    pub fn flush_pending_one(&mut self, peer: &PeerID) -> bool {
        let next = self
            .pending_inputs
            .get_mut(peer)
            .and_then(|q| q.pop_front());
        if let Some(input) = next {
            self.process(peer, input);
            true
        } else {
            false
        }
    }

    /// Fire the next pending timer for a peer.
    pub fn fire_timer_one(&mut self, peer: &PeerID) -> bool {
        let next = self.timers.get_mut(peer).and_then(|q| q.pop_front());
        if let Some(kind) = next {
            self.process(peer, Input::TimerFired { kind });
            true
        } else {
            false
        }
    }

    /// Drive a bootstrap handshake between two peers using their actual
    /// identity-signed offers/answers exchanged out-of-band.
    pub fn bootstrap_pair(&mut self, host: &PeerID, joiner: &PeerID) {
        self.process(host, Input::InitOpenOffer);
        self.flush_pending_one(host);

        let offer = self
            .events
            .iter()
            .rev()
            .find_map(|(p, e)| match e {
                ObservedEvent::OfferReady(pl) if p == host => Some(pl.clone()),
                _ => None,
            })
            .expect("host did not emit OfferReady");

        self.process(
            joiner,
            Input::InitHandshake {
                with: host.clone(),
                mode: HandshakeMode::Bootstrap,
                strategy: HandshakeStrategy::Joiner,
            },
        );
        self.process(
            joiner,
            Input::Handshake {
                from: host.clone(),
                event: HandshakeInput::Offer(offer),
            },
        );
        self.flush_pending_one(joiner);

        let answer = self
            .events
            .iter()
            .rev()
            .find_map(|(p, e)| match e {
                ObservedEvent::AnswerReady(pl) if p == joiner => Some(pl.clone()),
                _ => None,
            })
            .expect("joiner did not emit AnswerReady");

        self.process(
            host,
            Input::Handshake {
                from: joiner.clone(),
                event: HandshakeInput::Answer(answer),
            },
        );

        // Drain remaining DataChannelOpen inputs (one each side) and any
        // immediate fan-out the FSM emits on Connected.
        for _ in 0..4 {
            self.flush_pending_one(host);
            self.flush_pending_one(joiner);
        }
    }

    pub fn drain_to_quiescence(&mut self, max_steps: usize) {
        for _ in 0..max_steps {
            let mut any = false;
            let pairs: Vec<(PeerID, PeerID)> = self
                .msg_queue
                .iter()
                .filter(|(_, q)| !q.is_empty())
                .map(|(k, _)| k.clone())
                .collect();
            for (from, to) in pairs {
                if let Some(data) = self
                    .msg_queue
                    .get_mut(&(from.clone(), to.clone()))
                    .and_then(|q| q.pop_front())
                {
                    self.process(
                        &to,
                        Input::MessageReceived {
                            peer_from: from,
                            data,
                        },
                    );
                    any = true;
                }
            }
            let ids = self.ids();
            for id in &ids {
                any |= self.flush_pending_one(id);
            }
            for id in &ids {
                any |= self.fire_timer_one(id);
            }
            if !any {
                break;
            }
        }
    }

    /// Per-peer state invariants that must hold after every step.
    pub fn check_invariants(&self) {
        for (id, fsm) in &self.peers {
            let connected_set = fsm.connected_peers();
            assert_eq!(
                connected_set.len(),
                fsm.connected_number(),
                "peer {id}: connected_peers().len() != connected_number()"
            );
            for p in &connected_set {
                assert!(
                    fsm.is_connected(p),
                    "peer {id}: {p} in connected_peers() but is_connected returns false"
                );
            }
        }
    }

    /// True iff every peer is connected to every other peer.
    pub fn is_full_mesh(&self) -> bool {
        let n = self.peers.len();
        if n == 0 {
            return true;
        }
        self.peers.values().all(|p| p.connected_number() == n - 1)
    }

    /// Diagnostic: total queued wire messages, pending inputs, timers.
    pub fn quiescence_residue(&self) -> (usize, usize, usize) {
        let msgs: usize = self.msg_queue.values().map(|q| q.len()).sum();
        let pending: usize = self.pending_inputs.values().map(|q| q.len()).sum();
        let timers: usize = self.timers.values().map(|q| q.len()).sum();
        (msgs, pending, timers)
    }
}
