#[cfg(test)]
mod tests {
    use crate::{
        client_fsm::{ClientFSM, Host},
        state::{Input, Output, TransportInput},
    };

    fn drive_host_to_connected(fsm: &mut ClientFSM<Host>) {
        fsm.process::<&str>(Input::Transport(TransportInput::InitNegotiation));
        fsm.process::<&str>(Input::Transport(TransportInput::SDPOfferCreated {
            sdp: "mock-offer".into(),
        }));
        fsm.process::<&str>(Input::Transport(TransportInput::SDPAnswerReceived {
            sdp: "mock-answer".into(),
        }));
        fsm.process::<&str>(Input::Transport(TransportInput::DataChannelOpen));
    }

    #[test]
    fn message_delivery_only_when_connected() {
        let mut fsm = ClientFSM::<Host>::new();

        let out = fsm.process(Input::MessageReceived {
            peer_from: 1,
            data: "hello",
        });
        assert_eq!(out, None);

        drive_host_to_connected(&mut fsm);

        let out = fsm.process(Input::MessageReceived {
            peer_from: 1,
            data: "hello",
        });
        assert_eq!(
            out,
            Some(Output::ReceiveMessage {
                peer_from: 1,
                data: "hello"
            })
        );
    }

    #[test]
    fn send_blocked_when_not_connected() {
        let mut fsm = ClientFSM::<Host>::new();

        let out = fsm.process(Input::PeerSend {
            peer_to: 2,
            data: "msg",
        });
        assert_eq!(out, None);

        let out = fsm.process(Input::PeerBroadcast { data: "msg" });
        assert_eq!(out, None);

        drive_host_to_connected(&mut fsm);

        let out = fsm.process(Input::PeerSend {
            peer_to: 2,
            data: "msg",
        });
        assert_eq!(
            out,
            Some(Output::SendMessage {
                peer_to: 2,
                data: "msg"
            })
        );

        let out = fsm.process(Input::PeerBroadcast { data: "msg" });
        assert_eq!(out, Some(Output::Broadcast { data: "msg" }));
    }

    #[test]
    fn local_sdp_available_after_offer() {
        let mut fsm = ClientFSM::<Host>::new();

        assert_eq!(fsm.local_sdp(), None);

        fsm.process::<&str>(Input::Transport(TransportInput::InitNegotiation));
        assert_eq!(fsm.local_sdp(), None);

        fsm.process::<&str>(Input::Transport(TransportInput::SDPOfferCreated {
            sdp: "offer-sdp".into(),
        }));
        assert_eq!(fsm.local_sdp(), Some("offer-sdp".to_string()));
    }
}
