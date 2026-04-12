#[cfg(test)]
mod test {
    use crate::{
        HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeStrategy, Input, MeshNodeFSM,
        MsgPayload, Output, PeerID, SignalingPayload,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestMsg(String);

    fn alice() -> PeerID {
        PeerID::new("alice")
    }
    fn bob() -> PeerID {
        PeerID::new("bob")
    }

    fn drive_host_handshake(fsm: &mut MeshNodeFSM, remote: &PeerID) {
        // Create host FSM
        fsm.process::<TestMsg>(Input::InitHandshake {
            with: remote.clone(),
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Host,
        })
        .unwrap();

        let out = fsm
            .process::<TestMsg>(Input::Handshake {
                from: remote.clone(),
                event: HandshakeInput::Init,
            })
            .unwrap();
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::InitSDPOffer,
                ..
            }
        )));

        let out = fsm
            .process::<TestMsg>(Input::Handshake {
                from: remote.clone(),
                event: HandshakeInput::SignalingCreated(SignalingPayload::Offer("offer".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        let out = fsm
            .process::<TestMsg>(Input::Handshake {
                from: remote.clone(),
                event: HandshakeInput::Signaling(SignalingPayload::Answer("answer".into())),
            })
            .unwrap();
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::AcceptSDPAnswer { .. },
                ..
            }
        )));

        fsm.process::<TestMsg>(Input::Handshake {
            from: remote.clone(),
            event: HandshakeInput::DataChannelOpen,
        })
        .unwrap();

        assert!(fsm.is_connected(remote));
    }

    #[test]
    fn host_handshake_full_flow() {
        let mut mesh = MeshNodeFSM::new(alice());
        drive_host_handshake(&mut mesh, &bob());
        assert!(mesh.is_connected(&bob()));
    }

    #[test]
    fn joiner_handshake_full_flow() {
        let mut mesh = MeshNodeFSM::new(bob());

        // Create joiner FSM
        mesh.process::<TestMsg>(Input::InitHandshake {
            with: alice(),
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Joiner,
        })
        .unwrap();

        let out = mesh
            .process::<TestMsg>(Input::Handshake {
                from: alice(),
                event: HandshakeInput::Signaling(SignalingPayload::Offer("offer".into())),
            })
            .unwrap();
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::RequestSDPAnswer { .. },
                ..
            }
        )));

        let out = mesh
            .process::<TestMsg>(Input::Handshake {
                from: alice(),
                event: HandshakeInput::SignalingCreated(SignalingPayload::Answer("answer".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        mesh.process::<TestMsg>(Input::Handshake {
            from: alice(),
            event: HandshakeInput::DataChannelOpen,
        })
        .unwrap();

        assert!(mesh.is_connected(&alice()));
    }

    #[test]
    fn message_only_when_connected() {
        let mut mesh = MeshNodeFSM::new(alice());

        let out = mesh
            .process(Input::MessageReceived {
                peer_from: bob(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        drive_host_handshake(&mut mesh, &bob());

        let out = mesh
            .process(Input::MessageReceived {
                peer_from: bob(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Output::ReceiveMessage { .. }));
        if let Output::ReceiveMessage { data, .. } = &out[0] {
            match data {
                MsgPayload::User(TestMsg(text)) => assert_eq!(text, "hello"),
                _ => panic!("expected user payload"),
            }
        }
    }

    #[test]
    fn send_only_when_connected() {
        let mut mesh = MeshNodeFSM::new(alice());

        let out = mesh
            .process(Input::Send {
                peer_to: bob(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        drive_host_handshake(&mut mesh, &bob());

        let out = mesh
            .process(Input::Send {
                peer_to: bob(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Output::SendMessage { .. }));
    }

    #[test]
    fn peer_leaving_cleans_up() {
        let mut mesh = MeshNodeFSM::new(alice());
        drive_host_handshake(&mut mesh, &bob());
        assert!(mesh.is_connected(&bob()));

        let out = mesh
            .process::<TestMsg>(Input::PeerLeaving { peer: bob() })
            .unwrap();
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerDisconnected { .. }))
        );
        assert!(!mesh.is_connected(&bob()));
    }

    #[test]
    fn unknown_handshake_event_errors() {
        let mut mesh = MeshNodeFSM::new(alice());

        // No handshake context for bob → should error
        let result = mesh.process::<TestMsg>(Input::Handshake {
            from: bob(),
            event: HandshakeInput::DataChannelOpen,
        });
        assert!(result.is_err());
    }
}
