#[cfg(test)]
mod tests {
    use crate::{
        client_fsm::{ClientFSM, Host},
        state::{Input, Output, TransportInput},
    };

    #[test]
    fn message_delivery_only_when_connected() {
        let mut fsm = ClientFSM::<Host>::new();

        let out = fsm.process(Input::MessageReceived {
            peer_from: 1,
            data: "hello",
        });
        assert_eq!(out, None);

        fsm.process::<&str>(Input::Transport(TransportInput::InitNegotiation));
        fsm.process::<&str>(Input::Transport(TransportInput::SDPAnswerReceived {
            sdp: "mock_answer".into(),
        }));
        fsm.process::<&str>(Input::Transport(TransportInput::DataChannelOpen));

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

        // Drive to Connected
        fsm.process::<&str>(Input::Transport(TransportInput::InitNegotiation));
        fsm.process::<&str>(Input::Transport(TransportInput::SDPAnswerReceived {
            sdp: "mock_answer".into(),
        }));
        fsm.process::<&str>(Input::Transport(TransportInput::DataChannelOpen));

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
}
