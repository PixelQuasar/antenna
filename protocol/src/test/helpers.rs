#[macro_export]
macro_rules! extract_relay {
    ($outputs:expr, via: $via:expr) => {
        $outputs
            .iter()
            .find_map(|o| match o {
                Output::Relay { via, payload } if via == &$via => Some(payload.clone()),
                _ => None,
            })
            .expect(&format!("Expected relay via {:?}", $via))
    };
}

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

#[macro_export]
macro_rules! relay_through {
    ($relay_peer:expr, from: $from:expr, payload: $payload:expr) => {{
        let outputs = $relay_peer.process::<()>(Input::Relay {
            from: $from,
            payload: $payload,
        });
        outputs
    }};
}
