#[cfg(test)]
mod test {
    use crate::{
        HandshakeInput, Input, MeshNodeFSM, MsgPayload, Output,
        mesh::test::drive_bootstrap_handshake::drive_bootstrap_handshake,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestMsg(String);

    fn connected_pair() -> (MeshNodeFSM, MeshNodeFSM) {
        let mut a = MeshNodeFSM::new();
        let mut b = MeshNodeFSM::new();
        drive_bootstrap_handshake::<TestMsg>(&mut a, &mut b);
        (a, b)
    }

    #[test]
    fn bootstrap_handshake_establishes_connection() {
        let (a, b) = connected_pair();
        assert!(a.is_connected(b.id()));
        assert!(b.is_connected(a.id()));
    }

    #[test]
    fn message_only_when_connected() {
        let mut mesh = MeshNodeFSM::new();
        let mut other = MeshNodeFSM::new();
        let other_id = other.id().clone();

        let out = mesh
            .process(Input::MessageReceived {
                peer_from: other_id.clone(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        drive_bootstrap_handshake::<TestMsg>(&mut mesh, &mut other);

        let out = mesh
            .process(Input::MessageReceived {
                peer_from: other_id.clone(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Output::ReceiveMessage { .. }));
        if let Output::ReceiveMessage { data, .. } = &out[0] {
            match data {
                MsgPayload::User(TestMsg(text)) => assert_eq!(text, "hello"),
                _ => panic!("expected user payload"),
            }
        }
    }

    #[test]
    fn send_only_when_connected() {
        let mut mesh = MeshNodeFSM::new();
        let mut other = MeshNodeFSM::new();
        let other_id = other.id().clone();

        let out = mesh
            .process(Input::Send {
                peer_to: other_id.clone(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert!(out.is_empty());

        drive_bootstrap_handshake::<TestMsg>(&mut mesh, &mut other);

        let out = mesh
            .process(Input::Send {
                peer_to: other_id.clone(),
                data: MsgPayload::User(TestMsg("hello".into())),
            })
            .unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], Output::SendMessage { .. }));
    }

    #[test]
    fn peer_leaving_cleans_up() {
        let (mut mesh, other) = connected_pair();
        let other_id = other.id().clone();

        assert!(mesh.is_connected(&other_id));

        let out = mesh
            .process::<TestMsg>(Input::PeerLeaving {
                peer: other_id.clone(),
            })
            .unwrap();
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerDisconnected { .. }))
        );
        assert!(!mesh.is_connected(&other_id));
    }

    #[test]
    fn unknown_handshake_event_errors() {
        let mut mesh = MeshNodeFSM::new();
        let unknown_id = MeshNodeFSM::new().id().clone();

        let result = mesh.process::<TestMsg>(Input::Handshake {
            from: unknown_id,
            event: HandshakeInput::DataChannelOpen,
        });
        assert!(result.is_err());
    }
}
