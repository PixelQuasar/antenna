#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        HandshakeInput, HandshakeOutput, Input, MAX_RECONNECT_ATTEMPTS, MeshNodeFSM, MsgPayload,
        Output, PeerID, RelayPayload, Scheduled,
        test::{drive_bootstrap_handshake, join_mesh},
    };

    fn connected_pair() -> (MeshNodeFSM, MeshNodeFSM) {
        let mut a = MeshNodeFSM::new();
        let mut b = MeshNodeFSM::new();
        drive_bootstrap_handshake::<()>(&mut a, &mut b);
        (a, b)
    }

    fn connected_triple() -> HashMap<PeerID, MeshNodeFSM> {
        let mut peers = HashMap::new();
        let mut a = MeshNodeFSM::new();
        let a_id = a.id().clone();
        let mut b = MeshNodeFSM::new();
        let b_id = b.id().clone();
        drive_bootstrap_handshake::<()>(&mut a, &mut b);
        peers.insert(a_id, a);
        peers.insert(b_id.clone(), b);
        let c = MeshNodeFSM::new();
        let c_id = c.id().clone();
        peers.insert(c_id.clone(), c);
        join_mesh(&c_id, &b_id, &mut peers);
        peers
    }

    #[test]
    fn abrupt_disconnect_schedules_reconnect_timer() {
        let (mut a, b) = connected_pair();
        let b_id = b.id().clone();

        let out = a
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        assert!(
            out.iter().any(|o| matches!(
                o,
                Output::ScheduleTimer { kind: Scheduled::ReconnectAttempt { peer }, .. }
                if peer == &b_id
            )),
            "abrupt disconnect must schedule a reconnect timer"
        );
    }

    #[test]
    fn graceful_disconnect_does_not_schedule_reconnect() {
        let (mut a, b) = connected_pair();
        let b_id = b.id().clone();

        let out = a
            .process::<()>(Input::MessageReceived {
                peer_from: b_id.clone(),
                data: MsgPayload::Disconnect,
            })
            .unwrap();

        assert!(
            !out.iter()
                .any(|o| matches!(o, Output::ScheduleTimer { .. })),
            "graceful disconnect must not schedule reconnect"
        );
    }

    #[test]
    fn peer_leaving_input_does_not_schedule_reconnect() {
        let (mut a, b) = connected_pair();
        let b_id = b.id().clone();

        let out = a
            .process::<()>(Input::PeerLeaving { peer: b_id.clone() })
            .unwrap();

        assert!(
            !out.iter()
                .any(|o| matches!(o, Output::ScheduleTimer { .. })),
            "PeerLeaving must not schedule reconnect"
        );
    }

    #[test]
    fn stale_tick_after_leave_is_noop() {
        let (mut a, b) = connected_pair();
        let b_id = b.id().clone();

        let _ = a
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        let _ = a.process::<()>(Input::Leave).unwrap();

        let out = a
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();

        assert!(
            out.is_empty(),
            "stale tick after leave must produce no outputs"
        );
    }

    #[test]
    fn tick_with_no_relays_gives_up_silently() {
        let (mut a, b) = connected_pair();
        let b_id = b.id().clone();

        let _ = a
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        let out = a
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();

        assert!(
            !out.iter()
                .any(|o| matches!(o, Output::ScheduleTimer { .. })),
            "no further reconnect timer should fire once we give up",
        );
        assert!(
            !out.iter().any(|o| matches!(o, Output::SendMessage { .. })),
            "no relay introduction can be sent without a connected relay peer",
        );

        let stale = a
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();
        assert!(stale.is_empty(), "stale tick after give-up must be a no-op");
    }

    #[test]
    fn lower_id_initiates_as_host_via_relay() {
        let mut peers = connected_triple();
        let mut sorted_ids: Vec<PeerID> = peers.keys().cloned().collect();
        sorted_ids.sort();
        let a_id = sorted_ids[0].clone();
        let b_id = sorted_ids[1].clone();
        let c_id = sorted_ids[2].clone();

        let _ = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        let out = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();

        let relay_intro_sent = out.iter().any(|o| {
            matches!(
                o,
                Output::SendMessage {
                    peer_to,
                    data: MsgPayload::RelaySignalingTo { dst, data: RelayPayload::InitConnect(_) },
                } if peer_to == &c_id && dst == &b_id
            )
        });
        assert!(
            relay_intro_sent,
            "lower-id peer must send RelaySignalingTo via relay with InitConnect targeting the lost peer",
        );

        assert!(
            out.iter().any(|o| matches!(
                o,
                Output::Handshake { peer, event: HandshakeOutput::InitSDPOffer }
                if peer == &b_id
            )),
            "lower-id peer must locally drive its host handshake to InitSDPOffer",
        );

        assert!(
            out.iter().any(|o| matches!(
                o,
                Output::ScheduleTimer { kind: Scheduled::ReconnectAttempt { peer }, .. }
                if peer == &b_id
            )),
            "next reconnect tick must be babysat",
        );
    }

    #[test]
    fn higher_id_initiates_as_joiner_via_relay() {
        let mut peers = connected_triple();
        let mut sorted_ids: Vec<PeerID> = peers.keys().cloned().collect();
        sorted_ids.sort();
        let a_id = sorted_ids[0].clone();
        let b_id = sorted_ids[1].clone();
        let c_id = sorted_ids[2].clone();

        let _ = peers
            .get_mut(&b_id)
            .unwrap()
            .process::<()>(Input::Handshake {
                from: a_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        let out = peers
            .get_mut(&b_id)
            .unwrap()
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: a_id.clone() },
            })
            .unwrap();

        let relay_intro_sent = out.iter().any(|o| {
            matches!(
                o,
                Output::SendMessage {
                    peer_to,
                    data: MsgPayload::RelaySignalingTo { dst, data: RelayPayload::InitConnect(_) },
                } if peer_to == &c_id && dst == &a_id
            )
        });
        assert!(
            relay_intro_sent,
            "higher-id peer must send RelaySignalingTo via relay with InitConnect targeting the lost peer",
        );

        assert!(
            !out.iter().any(|o| matches!(
                o,
                Output::Handshake {
                    event: HandshakeOutput::InitSDPOffer,
                    ..
                }
            )),
            "higher-id (joiner role) peer must not start its own offer; it waits for the host",
        );
    }

    #[test]
    fn ticks_short_circuit_while_handshake_in_flight() {
        let mut peers = connected_triple();
        let mut sorted_ids: Vec<PeerID> = peers.keys().cloned().collect();
        sorted_ids.sort();
        let a_id = sorted_ids[0].clone();
        let b_id = sorted_ids[1].clone();

        let _ = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        let _ = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();

        for _ in 0..(MAX_RECONNECT_ATTEMPTS + 5) {
            let out = peers
                .get_mut(&a_id)
                .unwrap()
                .process::<()>(Input::TimerFired {
                    kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
                })
                .unwrap();
            assert!(
                out.is_empty(),
                "tick during in-flight handshake must be a no-op",
            );
        }
    }

    #[test]
    fn exhausted_attempts_clear_state_and_stop_scheduling() {
        let mut peers = connected_triple();
        let mut sorted_ids: Vec<PeerID> = peers.keys().cloned().collect();
        sorted_ids.sort();
        let a_id = sorted_ids[0].clone();
        let b_id = sorted_ids[1].clone();

        let _ = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::Handshake {
                from: b_id.clone(),
                event: HandshakeInput::ConnectionDropped,
            })
            .unwrap();

        for _ in 0..MAX_RECONNECT_ATTEMPTS {
            let _ = peers
                .get_mut(&a_id)
                .unwrap()
                .process::<()>(Input::TimerFired {
                    kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
                })
                .unwrap();
            let _ = peers
                .get_mut(&a_id)
                .unwrap()
                .process::<()>(Input::Handshake {
                    from: b_id.clone(),
                    event: HandshakeInput::ConnectionDropped,
                })
                .unwrap();
        }

        let out = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();
        assert!(
            !out.iter()
                .any(|o| matches!(o, Output::ScheduleTimer { .. })),
            "no further reconnect timer should be scheduled after exhaustion",
        );
        assert!(
            !out.iter().any(|o| matches!(o, Output::SendMessage { .. })),
            "no relay introduction should be sent on the exhausting tick",
        );

        let stale = peers
            .get_mut(&a_id)
            .unwrap()
            .process::<()>(Input::TimerFired {
                kind: Scheduled::ReconnectAttempt { peer: b_id.clone() },
            })
            .unwrap();
        assert!(stale.is_empty(), "post-exhaustion ticks must be no-ops");
    }
}
