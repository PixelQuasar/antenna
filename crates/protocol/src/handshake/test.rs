#[cfg(test)]
mod tests {
    use crate::handshake::{HandshakeInput, HandshakeOutput, HandshakeState, Host, Joiner};
    use crate::{Identity, SignalingPayload};

    fn mock_payload() -> SignalingPayload {
        let id = Identity::new();
        let sdp = "mock-sdp".to_string();
        let token = id.create_token(&sdp).unwrap();
        SignalingPayload {
            token,
            sdp,
            pubkey: id.pubkey(),
        }
    }

    #[test]
    fn host_smoke() {
        let mut host = Host::new();
        assert_eq!(*host.state(), HandshakeState::Idle);

        let out = host.process(HandshakeInput::Init).unwrap();
        assert_eq!(*host.state(), HandshakeState::CreatingOffer);
        assert_eq!(out, Some(HandshakeOutput::InitSDPOffer));

        let out = host
            .process(HandshakeInput::OfferCreated("mock-offer".into()))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
        assert_eq!(out, None);

        let out = host
            .process(HandshakeInput::Answer(mock_payload()))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForDataChannel);
        assert!(matches!(out, Some(HandshakeOutput::AcceptSDPAnswer(_))));

        let out = host.process(HandshakeInput::DataChannelOpen).unwrap();
        assert_eq!(*host.state(), HandshakeState::Connected);
        assert_eq!(out, Some(HandshakeOutput::Connected));
    }

    #[test]
    fn joiner_smoke() {
        let mut joiner = Joiner::new();
        assert_eq!(*joiner.state(), HandshakeState::Idle);

        let out = joiner
            .process(HandshakeInput::Offer(mock_payload()))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::CreatingAnswer);
        assert!(matches!(out, Some(HandshakeOutput::RequestSDPAnswer(_))));

        let out = joiner
            .process(HandshakeInput::AnswerCreated("mock-answer".into()))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
        assert_eq!(out, None);

        let out = joiner.process(HandshakeInput::DataChannelOpen).unwrap();
        assert_eq!(*joiner.state(), HandshakeState::Connected);
        assert_eq!(out, Some(HandshakeOutput::Connected));
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
        host.process(HandshakeInput::OfferCreated("mock-offer".into()))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);

        let result = host.process(HandshakeInput::Offer(mock_payload()));
        assert!(result.is_err());
    }

    #[test]
    fn host_transitions_to_waiting_for_answer_after_offer_created() {
        let mut host = Host::new();
        host.process(HandshakeInput::Init).unwrap();
        host.process(HandshakeInput::OfferCreated("v=0\r\noffer-sdp".into()))
            .unwrap();
        assert_eq!(*host.state(), HandshakeState::WaitingForAnswer);
    }

    #[test]
    fn joiner_transitions_to_waiting_for_dc_after_answer_created() {
        let mut joiner = Joiner::new();
        joiner
            .process(HandshakeInput::Offer(mock_payload()))
            .unwrap();
        joiner
            .process(HandshakeInput::AnswerCreated("v=0\r\nanswer-sdp".into()))
            .unwrap();
        assert_eq!(*joiner.state(), HandshakeState::WaitingForDataChannel);
    }
}
