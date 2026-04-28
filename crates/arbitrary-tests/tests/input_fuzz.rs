//! peer-level fuzzing

use antenna_arbitrary_tests::{arb_input, arb_peer_id, identity_peer_ids};
use antenna_protocol::{Input, MeshNodeFSM};
use proptest::prelude::*;

const RANDOM_POOL_SIZE: usize = 2;
const SEQ_MAX: usize = 50;

fn arb_pool_and_inputs() -> impl Strategy<Value = Vec<Input<()>>> {
    let real = identity_peer_ids();
    proptest::collection::vec(arb_peer_id(), RANDOM_POOL_SIZE..=RANDOM_POOL_SIZE).prop_flat_map(
        move |random| {
            let mut combined = real.clone();
            combined.extend(random);
            proptest::collection::vec(arb_input(combined), 0..=SEQ_MAX)
        },
    )
}

fn check_invariants(fsm: &MeshNodeFSM) {
    let connected_set = fsm.connected_peers();
    assert_eq!(
        connected_set.len(),
        fsm.connected_number(),
        "connected_peers().len() != connected_number()"
    );
    for p in &connected_set {
        assert!(
            fsm.is_connected(p),
            "peer {p} reported connected via set but is_connected returns false"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]


    #[test]
    fn fsm_survives_random_input_sequence(inputs in arb_pool_and_inputs()) {
        let mut fsm = MeshNodeFSM::new();
        for input in inputs {
            let _ = fsm.process::<()>(input);
            check_invariants(&fsm);
        }
    }


    #[test]
    fn leave_clears_state(
        prefix in proptest::collection::vec(
            arb_input(identity_peer_ids().into_iter().take(2).collect()),
            0..30,
        ),
    ) {
        let mut fsm = MeshNodeFSM::new();
        for input in prefix {
            let _ = fsm.process::<()>(input);
        }
        let _ = fsm.process::<()>(Input::Leave);
        prop_assert_eq!(fsm.connected_number(), 0);
        prop_assert!(fsm.connected_peers().is_empty());
    }
}
