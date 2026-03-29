#[cfg(test)]
mod tests {
    use crate::transport::{Host, Joiner, TransportInput, TransportOutput, TransportState};

    #[test]
    fn host_smoke() {
        let mut host = Host::new();
        assert_eq!(*host.state(), TransportState::Idle);

        let out = host.process(TransportInput::InitNegotiation);
        assert_eq!(*host.state(), TransportState::CreatingOffer);
        assert_eq!(out, Some(TransportOutput::InitSDPOffer));

        let out = host.process(TransportInput::SDPOfferCreated {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);
        assert_eq!(out, None);

        let out = host.process(TransportInput::SDPAnswerReceived {
            sdp: "mock-answer".into(),
        });
        assert_eq!(*host.state(), TransportState::WaitingForDataChannel);
        assert_eq!(
            out,
            Some(TransportOutput::AcceptSDPAnswer {
                sdp: "mock-answer".into()
            })
        );

        let out = host.process(TransportInput::DataChannelOpen);
        assert_eq!(*host.state(), TransportState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn joiner_smoke() {
        let mut joiner = Joiner::new();
        assert_eq!(*joiner.state(), TransportState::Idle);

        let out = joiner.process(TransportInput::SDPOfferReceived {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*joiner.state(), TransportState::CreatingAnswer);
        assert_eq!(
            out,
            Some(TransportOutput::InitSDPAnswer {
                offer_sdp: "mock-offer".into()
            })
        );

        let out = joiner.process(TransportInput::SDPAnswerCreated {
            sdp: "mock-answer".into(),
        });
        assert_eq!(*joiner.state(), TransportState::WaitingForDataChannel);
        assert_eq!(out, None);

        let out = joiner.process(TransportInput::DataChannelOpen);
        assert_eq!(*joiner.state(), TransportState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn host_disconnect_mid_handshake() {
        let mut host = Host::new();
        host.process(TransportInput::InitNegotiation);
        assert_eq!(*host.state(), TransportState::CreatingOffer);

        let out = host.process(TransportInput::Disconnected);
        assert_eq!(*host.state(), TransportState::Closed);
        assert_eq!(out, Some(TransportOutput::Close));
    }

    #[test]
    fn invalid_input_ignored() {
        let mut host = Host::new();
        host.process(TransportInput::InitNegotiation);
        host.process(TransportInput::SDPOfferCreated {
            sdp: "mock-offer".into(),
        });
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);

        let out = host.process(TransportInput::SDPOfferReceived { sdp: "mock".into() });
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);
        assert_eq!(out, None);
    }

    #[test]
    fn host_transitions_to_waiting_for_answer_after_offer_created() {
        let mut host = Host::new();
        host.process(TransportInput::InitNegotiation);
        host.process(TransportInput::SDPOfferCreated {
            sdp: "v=0\r\noffer-sdp".into(),
        });
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);
    }

    #[test]
    fn joiner_transitions_to_waiting_for_dc_after_answer_created() {
        let mut joiner = Joiner::new();
        joiner.process(TransportInput::SDPOfferReceived {
            sdp: "mock-offer".into(),
        });
        joiner.process(TransportInput::SDPAnswerCreated {
            sdp: "v=0\r\nanswer-sdp".into(),
        });
        assert_eq!(*joiner.state(), TransportState::WaitingForDataChannel);
    }
}
