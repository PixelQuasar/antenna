#[cfg(test)]
mod test {
    use crate::{
        HandshakeInput, HandshakeMode, HandshakeStrategy, Input, MeshNodeFSM, Output, Scheduled,
        test::drive_bootstrap_handshake,
    };

    #[test]
    fn relay_via_drop_cleans_up_orphan_handshakes() {
        let mut alice = MeshNodeFSM::new();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();
        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        let charlie_id = MeshNodeFSM::new().id().clone();

        alice
            .process::<()>(Input::InitHandshake {
                with: charlie_id.clone(),
                mode: HandshakeMode::Relay(bob_id.clone()),
                strategy: HandshakeStrategy::Joiner,
            })
            .unwrap();

        let outs = alice
            .process::<()>(Input::Handshake {
                from: bob_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        assert!(!alice.is_connected(&bob_id), "bob should be removed");
        assert!(
            outs.iter()
                .any(|o| matches!(o, Output::PeerLost { peer } if peer == &bob_id)),
            "bob (was connected) should emit PeerLost"
        );
        assert!(
            outs.iter().any(|o| matches!(
                o,
                Output::ScheduleTimer { kind: Scheduled::ReconnectAttempt { peer }, .. }
                    if peer == &bob_id
            )),
            "bob's reconnect attempt should be scheduled"
        );
        assert!(
            !outs
                .iter()
                .any(|o| matches!(o, Output::PeerLost { peer } if peer == &charlie_id)),
            "charlie never reached Connected — must NOT emit PeerLost"
        );
        assert!(
            outs.iter().any(|o| matches!(
                o,
                Output::ScheduleTimer { kind: Scheduled::ReconnectAttempt { peer }, .. }
                    if peer == &charlie_id
            )),
            "charlie's orphan handshake should queue a reconnect attempt"
        );
    }

    #[test]
    fn relay_via_drop_leaves_unrelated_relays_alone() {
        let mut alice = MeshNodeFSM::new();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();
        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        let other_relay_id = MeshNodeFSM::new().id().clone();
        let dave_id = MeshNodeFSM::new().id().clone();

        alice
            .process::<()>(Input::InitHandshake {
                with: dave_id.clone(),
                mode: HandshakeMode::Relay(other_relay_id),
                strategy: HandshakeStrategy::Joiner,
            })
            .unwrap();

        let outs = alice
            .process::<()>(Input::Handshake {
                from: bob_id,
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        assert!(
            !outs.iter().any(|o| matches!(
                o,
                Output::ScheduleTimer { kind: Scheduled::ReconnectAttempt { peer }, .. }
                    if peer == &dave_id
            )),
            "dave's relay was via a different peer; must not be cleaned up"
        );
    }
}
