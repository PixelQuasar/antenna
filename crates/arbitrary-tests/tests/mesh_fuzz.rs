//! mesh-level fuzzing

use antenna_arbitrary_tests::MeshSim;
use antenna_protocol::{Input, MsgPayload};
use proptest::prelude::*;

const N_PEERS: usize = 10;
const SCHEDULE_LEN: usize = 600;
const DRAIN_BUDGET: usize = 10_000;

/// Chain bootstrap `(0,1), (1,2), …, (N-2, N-1)`
fn bootstrap_chain(sim: &mut MeshSim, ids: &[antenna_protocol::PeerID]) {
    for i in 0..ids.len().saturating_sub(1) {
        sim.bootstrap_pair(&ids[i], &ids[i + 1]);
    }
}

#[derive(Debug, Clone)]
enum Step {
    DeliverAt(usize),
    FlushPending(usize),
    FireTimer(usize),
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        any::<usize>().prop_map(Step::DeliverAt),
        (0..N_PEERS).prop_map(Step::FlushPending),
        (0..N_PEERS).prop_map(Step::FireTimer),
    ]
}

#[derive(Debug, Clone)]
enum DisruptiveAction {
    None,
    Leave(usize),
    Broadcast(usize),
    PeerLeavingFrom { peer: usize, missing: usize },
}

fn arb_disruptive() -> impl Strategy<Value = DisruptiveAction> {
    prop_oneof![
        20 => Just(DisruptiveAction::None),
        1 => (0..N_PEERS).prop_map(DisruptiveAction::Leave),
        3 => (0..N_PEERS).prop_map(DisruptiveAction::Broadcast),
        2 => (0..N_PEERS, 0..N_PEERS)
            .prop_map(|(peer, missing)| DisruptiveAction::PeerLeavingFrom { peer, missing }),
    ]
}

fn run_step(sim: &mut MeshSim, ids: &[antenna_protocol::PeerID], step: &Step) {
    match step {
        Step::DeliverAt(idx) => {
            sim.deliver_msg_at(*idx);
        }
        Step::FlushPending(i) => {
            sim.flush_pending_one(&ids[*i]);
        }
        Step::FireTimer(i) => {
            sim.fire_timer_one(&ids[*i]);
        }
    }
}

fn apply_disruptive(
    sim: &mut MeshSim,
    ids: &[antenna_protocol::PeerID],
    action: &DisruptiveAction,
) {
    match action {
        DisruptiveAction::None => {}
        DisruptiveAction::Leave(i) => {
            sim.process(&ids[*i], Input::Leave);
        }
        DisruptiveAction::Broadcast(i) => {
            sim.process(
                &ids[*i],
                Input::Broadcast {
                    data: MsgPayload::User(()),
                },
            );
        }
        DisruptiveAction::PeerLeavingFrom { peer, missing } => {
            sim.process(
                &ids[*peer],
                Input::PeerLeaving {
                    peer: ids[*missing].clone(),
                },
            );
        }
    }
}

/// Deterministic repro for debugging: chain bootstrap N=10 with empty
/// random schedule, then drain. Dumps full per-peer connection state on
/// failure so we can see which handshakes are stuck and in what state.
#[test]
fn debug_chain_bootstrap_convergence() {
    let mut sim = MeshSim::new(N_PEERS);
    let ids = sim.ids();
    bootstrap_chain(&mut sim, &ids);
    sim.drain_to_quiescence(DRAIN_BUDGET);

    if !sim.is_full_mesh() {
        eprintln!("=== chain bootstrap N={} did not converge ===", N_PEERS);
        eprintln!("residue (msgs, pending, timers) = {:?}", sim.quiescence_residue());
        for (idx, id) in ids.iter().enumerate() {
            let fsm = &sim.peers[id];
            let mut snapshot = fsm.connections_snapshot();
            snapshot.sort_by_key(|(p, _, _)| p.clone());
            let connected = fsm.connected_number();
            eprintln!(
                "peer[{idx}] {id}: connected={connected}, pending_handshakes={}, total_ctx={}",
                fsm.pending_handshakes_len(),
                snapshot.len(),
            );
            for (peer, state, mode) in snapshot {
                let peer_idx = ids.iter().position(|x| x == &peer).map_or(-1i32, |i| i as i32);
                eprintln!("    -> peer[{peer_idx}] state={:?} mode={:?}", state, mode);
            }
        }
        panic!("not full mesh");
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_shrink_iters: 2048,
        ..ProptestConfig::default()
    })]


    #[test]
    fn mesh_converges_under_random_schedule(
        schedule in proptest::collection::vec(arb_step(), 0..=SCHEDULE_LEN)
    ) {
        let mut sim = MeshSim::new(N_PEERS);
        let ids = sim.ids();

        bootstrap_chain(&mut sim, &ids);
        sim.check_invariants();

        for step in &schedule {
            run_step(&mut sim, &ids, step);
            sim.check_invariants();
        }

        sim.drain_to_quiescence(DRAIN_BUDGET);
        sim.check_invariants();
        let residue = sim.quiescence_residue();
        if !sim.is_full_mesh() {
            let stuck: Vec<(antenna_protocol::PeerID, antenna_protocol::PeerID, antenna_protocol::HandshakeState, antenna_protocol::HandshakeMode)> = sim
                .peers
                .iter()
                .flat_map(|(self_id, fsm)| {
                    fsm.connections_snapshot()
                        .into_iter()
                        .filter_map(|(p, s, m)| {
                            if matches!(s, antenna_protocol::HandshakeState::Connected) {
                                None
                            } else {
                                Some((self_id.clone(), p, s, m))
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            prop_assert!(
                false,
                "mesh did not converge: residue={residue:?}, stuck handshakes:\n{stuck:#?}"
            );
        }
    }

    #[test]
    fn mesh_invariants_under_random_actions(
        ops in proptest::collection::vec(
            (arb_step(), arb_disruptive()),
            0..=SCHEDULE_LEN,
        )
    ) {
        let mut sim = MeshSim::new(N_PEERS);
        let ids = sim.ids();

        bootstrap_chain(&mut sim, &ids);
        sim.check_invariants();

        for (step, disruptive) in &ops {
            run_step(&mut sim, &ids, step);
            sim.check_invariants();
            apply_disruptive(&mut sim, &ids, disruptive);
            sim.check_invariants();
        }

        sim.drain_to_quiescence(DRAIN_BUDGET);
        sim.check_invariants();
    }
}
