#[cfg(test)]
mod test {
    use crate::{HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, Output, PeerID};

    fn alice() -> PeerID {
        PeerID::new("alice")
    }
    fn bob() -> PeerID {
        PeerID::new("bob")
    }

    fn drive_host_handshake(fsm: &mut MeshNodeFSM, remote: &PeerID) {
        let out = fsm.process::<&str>(Input::Handshake {
            from: remote.clone(),
            event: HandshakeInput::InitNegotiation,
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::InitSDPOffer,
                ..
            }
        )));

        let out = fsm.process::<&str>(Input::Handshake {
            from: remote.clone(),
            event: HandshakeInput::SDPOfferCreated {
                sdp: "offer".into(),
            },
        });
        assert!(out.is_empty());

        let out = fsm.process::<&str>(Input::Handshake {
            from: remote.clone(),
            event: HandshakeInput::SDPAnswerReceived {
                sdp: "answer".into(),
            },
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::AcceptSDPAnswer { .. },
                ..
            }
        )));

        fsm.process::<&str>(Input::Handshake {
            from: remote.clone(),
            event: HandshakeInput::DataChannelOpen,
        });

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

        let out = mesh.process::<&str>(Input::Handshake {
            from: alice(),
            event: HandshakeInput::SDPOfferReceived {
                sdp: "offer".into(),
            },
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Handshake {
                event: HandshakeOutput::RequestSDPAnswer { .. },
                ..
            }
        )));

        let out = mesh.process::<&str>(Input::Handshake {
            from: alice(),
            event: HandshakeInput::SDPAnswerCreated {
                sdp: "answer".into(),
            },
        });
        assert!(out.is_empty());

        mesh.process::<&str>(Input::Handshake {
            from: alice(),
            event: HandshakeInput::DataChannelOpen,
        });

        assert!(mesh.is_connected(&alice()));
    }

    #[test]
    fn message_only_when_connected() {
        let mut mesh = MeshNodeFSM::new(alice());

        let out = mesh.process(Input::MessageReceived {
            peer_from: bob(),
            data: "hello",
        });
        assert!(out.is_empty());

        drive_host_handshake(&mut mesh, &bob());

        let out = mesh.process(Input::MessageReceived {
            peer_from: bob(),
            data: "hello",
        });
        assert_eq!(out.len(), 1);
        assert!(matches!(
            &out[0],
            Output::ReceiveMessage { data: "hello", .. }
        ));
    }

    #[test]
    fn send_only_when_connected() {
        let mut mesh = MeshNodeFSM::new(alice());

        let out = mesh.process(Input::PeerSend {
            peer_to: bob(),
            data: "msg",
        });
        assert!(out.is_empty());

        drive_host_handshake(&mut mesh, &bob());

        let out = mesh.process(Input::PeerSend {
            peer_to: bob(),
            data: "msg",
        });
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Output::SendMessage { .. }));
    }

    #[test]
    fn peer_leaving_cleans_up() {
        let mut mesh = MeshNodeFSM::new(alice());
        drive_host_handshake(&mut mesh, &bob());
        assert!(mesh.is_connected(&bob()));

        let out = mesh.process::<&str>(Input::PeerLeaving { peer: bob() });
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerDisconnected { .. }))
        );
        assert!(!mesh.is_connected(&bob()));
    }

    #[test]
    fn unknown_handshake_event_ignored() {
        let mut mesh = MeshNodeFSM::new(alice());

        let out = mesh.process::<&str>(Input::Handshake {
            from: bob(),
            event: HandshakeInput::DataChannelOpen,
        });
        assert!(out.is_empty());
    }
}
