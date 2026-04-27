#[macro_export]
macro_rules! assert_handshake_event {
        ($outputs:expr, peer: $peer:expr, event: $event_pattern:pat) => {
            assert!(
                $outputs.iter().any(|o| matches!(
                    o,
                    Output::Handshake { peer, event: $event_pattern } if peer == &$peer
                )),
                "Expected handshake event {} for peer {:?}",
                stringify!($event_pattern),
                $peer
            );
        };
    }
