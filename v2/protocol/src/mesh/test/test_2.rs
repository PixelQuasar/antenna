#[cfg(test)]
mod test {
    use crate::{
        mesh::{MeshFSM, PeerID},
        state::{Input, Output},
        transport::{TransportInput, TransportOutput},
    };

    fn alice() -> PeerID {
        PeerID::new("alice")
    }
    fn bob() -> PeerID {
        PeerID::new("bob")
    }

    fn drive_host_handshake(fsm: &mut MeshFSM, remote: &PeerID) {
        let out = fsm.process::<&str>(Input::Transport {
            peer: remote.clone(),
            event: TransportInput::InitNegotiation,
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Transport {
                event: TransportOutput::InitSDPOffer,
                ..
            }
        )));

        let out = fsm.process::<&str>(Input::Transport {
            peer: remote.clone(),
            event: TransportInput::SDPOfferCreated {
                sdp: "offer".into(),
            },
        });
        assert!(out.is_empty());

        let out = fsm.process::<&str>(Input::Transport {
            peer: remote.clone(),
            event: TransportInput::SDPAnswerReceived {
                sdp: "answer".into(),
            },
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Transport {
                event: TransportOutput::AcceptSDPAnswer { .. },
                ..
            }
        )));

        let out = fsm.process::<&str>(Input::Transport {
            peer: remote.clone(),
            event: TransportInput::DataChannelOpen,
        });
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerConnected { .. }))
        );
    }

    #[test]
    fn host_handshake_full_flow() {
        let mut mesh = MeshFSM::new(alice());
        drive_host_handshake(&mut mesh, &bob());
        assert!(mesh.is_connected(&bob()));
    }

    #[test]
    fn joiner_handshake_full_flow() {
        let mut mesh = MeshFSM::new(bob());

        let out = mesh.process::<&str>(Input::Transport {
            peer: alice(),
            event: TransportInput::SDPOfferReceived {
                sdp: "offer".into(),
            },
        });
        assert!(out.iter().any(|o| matches!(
            o,
            Output::Transport {
                event: TransportOutput::InitSDPAnswer { .. },
                ..
            }
        )));

        let out = mesh.process::<&str>(Input::Transport {
            peer: alice(),
            event: TransportInput::SDPAnswerCreated {
                sdp: "answer".into(),
            },
        });
        assert!(out.is_empty());

        let out = mesh.process::<&str>(Input::Transport {
            peer: alice(),
            event: TransportInput::DataChannelOpen,
        });
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerConnected { .. }))
        );
        assert!(mesh.is_connected(&alice()));
    }

    #[test]
    fn message_only_when_connected() {
        let mut mesh = MeshFSM::new(alice());

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
        let mut mesh = MeshFSM::new(alice());

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
        let mut mesh = MeshFSM::new(alice());
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
    fn unknown_transport_event_ignored() {
        let mut mesh = MeshFSM::new(alice());

        let out = mesh.process::<&str>(Input::Transport {
            peer: bob(),
            event: TransportInput::DataChannelOpen,
        });
        assert!(out.is_empty());
    }
}
