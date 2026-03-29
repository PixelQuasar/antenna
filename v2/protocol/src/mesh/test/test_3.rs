#[cfg(test)]
mod test {
    use crate::{Input, MeshFSM, Output, TransportInput, TransportOutput, mesh::PeerID};

    #[test]
    fn three_peers_mesh() {
        let mut alice = MeshFSM::new(PeerID::new("alice"));
        let mut bob = MeshFSM::new(PeerID::new("bob"));
        let mut charlie = MeshFSM::new(PeerID::new("charlie"));

        let out = alice.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::InitNegotiation,
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPOffer,
                ..
            }
        ));

        alice.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::SDPOfferCreated {
                sdp: "alice-offer".into(),
            },
        });

        let out = bob.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::SDPOfferReceived {
                sdp: "alice-offer".into(),
            },
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPAnswer { .. },
                ..
            }
        ));

        bob.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::SDPAnswerCreated {
                sdp: "bob-answer".into(),
            },
        });

        let out = alice.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::SDPAnswerReceived {
                sdp: "bob-answer".into(),
            },
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::AcceptSDPAnswer { .. },
                ..
            }
        ));

        let out = alice.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::DataChannelOpen,
        });
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerConnected { .. }))
        );

        let out = bob.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::DataChannelOpen,
        });
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerConnected { .. }))
        );

        assert!(alice.is_connected(&PeerID::new("bob")));
        assert!(bob.is_connected(&PeerID::new("alice")));

        let out = bob.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::InitNegotiation,
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPOffer,
                ..
            }
        ));

        bob.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::SDPOfferCreated {
                sdp: "bob-offer-for-charlie".into(),
            },
        });

        let out = charlie.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::SDPOfferReceived {
                sdp: "bob-offer-for-charlie".into(),
            },
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPAnswer { .. },
                ..
            }
        ));

        charlie.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::SDPAnswerCreated {
                sdp: "charlie-answer".into(),
            },
        });

        bob.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::SDPAnswerReceived {
                sdp: "charlie-answer".into(),
            },
        });

        bob.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::DataChannelOpen,
        });
        charlie.process::<()>(Input::Transport {
            peer: PeerID::new("bob"),
            event: TransportInput::DataChannelOpen,
        });

        assert!(bob.is_connected(&PeerID::new("charlie")));
        assert!(charlie.is_connected(&PeerID::new("bob")));

        let out = alice.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::InitNegotiation,
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPOffer,
                ..
            }
        ));

        alice.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::SDPOfferCreated {
                sdp: "alice-offer-for-charlie".into(),
            },
        });

        let out = charlie.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::SDPOfferReceived {
                sdp: "alice-offer-for-charlie".into(),
            },
        });
        assert!(matches!(
            out[0],
            Output::Transport {
                event: TransportOutput::InitSDPAnswer { .. },
                ..
            }
        ));

        charlie.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::SDPAnswerCreated {
                sdp: "charlie-answer-for-alice".into(),
            },
        });

        alice.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::SDPAnswerReceived {
                sdp: "charlie-answer-for-alice".into(),
            },
        });

        alice.process::<()>(Input::Transport {
            peer: PeerID::new("charlie"),
            event: TransportInput::DataChannelOpen,
        });
        charlie.process::<()>(Input::Transport {
            peer: PeerID::new("alice"),
            event: TransportInput::DataChannelOpen,
        });

        assert!(alice.is_connected(&PeerID::new("bob")));
        assert!(alice.is_connected(&PeerID::new("charlie")));
        assert!(bob.is_connected(&PeerID::new("alice")));
        assert!(bob.is_connected(&PeerID::new("charlie")));
        assert!(charlie.is_connected(&PeerID::new("alice")));
        assert!(charlie.is_connected(&PeerID::new("bob")));

        assert_eq!(alice.connected_peers().len(), 2);
        assert_eq!(bob.connected_peers().len(), 2);
        assert_eq!(charlie.connected_peers().len(), 2);
    }
}
