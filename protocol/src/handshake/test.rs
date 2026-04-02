#[cfg(test)]
mod tests {
    use crate::handshake::{Host, Joiner, HandshakeInput, HandshakeOutput, HandshakeState};

    #[test]
    fn host_smoke() {
        let mut host = Host::new();
        assert_eq!(*host.state(), HandshakeState::Idle);

        let out = host.process(HandshakeInput::InitNegotiation);
        assert_eq!(*host.state(), HandshakeState::CreatingOffer);
        assert_eq!(out, Some(HandshakeOutput::InitSDPOffer));

        let out = host.process(HandshakeInput::SDPOfferCreated {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
        assert_eq!(out, None);

        let out = host.process(HandshakeInput::SDPAnswerReceived {
            sdp: "mock-answer".into(),
        });
        assert_eq!(*host.state(), HandshakeState::WaitingForDataChannel);
        assert_eq!(
            out,
            Some(HandshakeOutput::AcceptSDPAnswer {
                sdp: "mock-answer".into()
            })
        );

        let out = host.process(HandshakeInput::DataChannelOpen);
        assert_eq!(*host.state(), HandshakeState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn joiner_smoke() {
        let mut joiner = Joiner::new();
        assert_eq!(*joiner.state(), HandshakeState::Idle);

        let out = joiner.process(HandshakeInput::SDPOfferReceived {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*joiner.state(), HandshakeState::CreatingAnswer);
        assert_eq!(
            out,
            Some(HandshakeOutput::InitSDPAnswer {
                offer_sdp: "mock-offer".into()
            })
        );

        let out = joiner.process(HandshakeInput::SDPAnswerCreated {
            sdp: "mock-answer".into(),
        });
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
        assert_eq!(out, None);

        let out = joiner.process(HandshakeInput::DataChannelOpen);
        assert_eq!(*joiner.state(), HandshakeState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn host_disconnect_mid_handshake() {
        let mut host = Host::new();
        host.process(HandshakeInput::InitNegotiation);
        assert_eq!(*host.state(), HandshakeState::CreatingOffer);

        let out = host.process(HandshakeInput::Disconnected);
        assert_eq!(*host.state(), HandshakeState::Closed);
        assert_eq!(out, Some(HandshakeOutput::Close));
    }

    #[test]
    fn invalid_input_ignored() {
        let mut host = Host::new();
        host.process(HandshakeInput::InitNegotiation);
        host.process(HandshakeInput::SDPOfferCreated {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);

        let out = host.process(HandshakeInput::SDPOfferReceived { sdp: "mock".into() });
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
        assert_eq!(out, None);
    }

    #[test]
    fn host_transitions_to_waiting_for_answer_after_offer_created() {
        let mut host = Host::new();
        host.process(HandshakeInput::InitNegotiation);
        host.process(HandshakeInput::SDPOfferCreated {
            sdp: "v=0\r\noffer-sdp".into(),
        });
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
    }

    #[test]
    fn joiner_transitions_to_waiting_for_dc_after_answer_created() {
        let mut joiner = Joiner::new();
        joiner.process(HandshakeInput::SDPOfferReceived {
            sdp: "mock-offer".into(),
        });
        joiner.process(HandshakeInput::SDPAnswerCreated {
            sdp: "v=0\r\nanswer-sdp".into(),
        });
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
    }
}
