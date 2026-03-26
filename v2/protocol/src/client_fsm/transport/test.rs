#[cfg(test)]
mod tests {
    use crate::{
        client_fsm::transport::{Host, Joiner, TransportFSM, host},
        state::{TransportInput, TransportOutput, TransportState},
    };

    #[test]
    fn host_smoke() {
        let mut host = Host::new();
        assert_eq!(*host.state(), TransportState::Idle);

        let out = host.process(TransportInput::InitNegotiation);
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);
        assert_eq!(out, Some(TransportOutput::InitSDPOffer));

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
        assert_eq!(*joiner.state(), TransportState::WaitingForDataChannel);
        assert_eq!(
            out,
            Some(TransportOutput::InitSDPAnswer {
                offer_sdp: "mock-offer".into()
            })
        );

        let out = joiner.process(TransportInput::DataChannelOpen);
        assert_eq!(*joiner.state(), TransportState::Connected);
        assert_eq!(out, None);
    }

    #[test]
    fn host_disconnect_mid_handshake() {
        let mut host = Host::new();
        host.process(TransportInput::InitNegotiation);
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);

        let out = host.process(TransportInput::Disconnected);
        assert_eq!(*host.state(), TransportState::Closed);
        assert_eq!(out, Some(TransportOutput::Close));
    }

    #[test]
    fn invalid_input_ignored() {
        let mut host = Host::new();
        host.process(TransportInput::InitNegotiation);
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);

        let out = host.process(TransportInput::SDPOfferReceived { sdp: "mock".into() });
        assert_eq!(*host.state(), TransportState::WaitingForAnswer);
        assert_eq!(out, None);
    }
}
