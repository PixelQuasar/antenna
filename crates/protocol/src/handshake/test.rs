#[cfg(test)]
mod tests {
    use crate::SignalingPayload;
    use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState, Host, Joiner};

    #[test]
    fn host_smoke() {
        let mut host = Host::new();
        assert_eq!(*host.state(), HandshakeState::Idle);

        let out = host.process(HandshakeInput::Init).unwrap();
        assert_eq!(*host.state(), HandshakeState::CreatingOffer);
        assert_eq!(out, Some(HandshakeOutput::InitSDPOffer));

        let out = host
            .process(HandshakeInput::SignalingCreated(SignalingPayload::Offer(
                "mock-offer".into(),
            )))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
        assert_eq!(out, None);

        let out = host
            .process(HandshakeInput::Signaling(SignalingPayload::Answer(
                "mock-answer".into(),
            )))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForDataChannel);
        assert_eq!(
            out,
            Some(HandshakeOutput::AcceptSDPAnswer {
                answer: "mock-answer".into()
            })
        );

        let out = host.process(HandshakeInput::DataChannelOpen).unwrap();
        assert_eq!(*host.state(), HandshakeState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn joiner_smoke() {
        let mut joiner = Joiner::new();
        assert_eq!(*joiner.state(), HandshakeState::Idle);

        let out = joiner
            .process(HandshakeInput::Signaling(SignalingPayload::Offer(
                "mock-offer".into(),
            )))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::CreatingAnswer);
        assert_eq!(
            out,
            Some(HandshakeOutput::RequestSDPAnswer {
                offer: "mock-offer".into()
            })
        );

        let out = joiner
            .process(HandshakeInput::SignalingCreated(SignalingPayload::Answer(
                "mock-answer".into(),
            )))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
        assert_eq!(out, None);

        let out = joiner.process(HandshakeInput::DataChannelOpen).unwrap();
        assert_eq!(*joiner.state(), HandshakeState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn host_disconnect_mid_handshake() {
        let mut host = Host::new();
        host.process(HandshakeInput::Init).unwrap();
        assert_eq!(*host.state(), HandshakeState::CreatingOffer);

        let out = host.process(HandshakeInput::Disconnected).unwrap();
        assert_eq!(*host.state(), HandshakeState::Closed);
        assert_eq!(out, Some(HandshakeOutput::Close));
    }

    #[test]
    fn invalid_input_returns_error() {
        let mut host = Host::new();
        host.process(HandshakeInput::Init).unwrap();
        host.process(HandshakeInput::SignalingCreated(SignalingPayload::Offer(
            "mock-offer".into(),
        )))
        .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);

        // Feeding an Offer when expecting Answer should error
        let result = host.process(HandshakeInput::Signaling(SignalingPayload::Offer(
            "mock".into(),
        )));
        assert!(result.is_err());
    }

    #[test]
    fn host_transitions_to_waiting_for_answer_after_offer_created() {
        let mut host = Host::new();
        host.process(HandshakeInput::Init).unwrap();
        host.process(HandshakeInput::SignalingCreated(SignalingPayload::Offer(
            "v=0\r\noffer-sdp".into(),
        )))
        .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
    }

    #[test]
    fn joiner_transitions_to_waiting_for_dc_after_answer_created() {
        let mut joiner = Joiner::new();
        joiner
            .process(HandshakeInput::Signaling(SignalingPayload::Offer(
                "mock-offer".into(),
            )))
            .unwrap();
        joiner
            .process(HandshakeInput::SignalingCreated(SignalingPayload::Answer(
                "v=0\r\nanswer-sdp".into(),
            )))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
    }
}
