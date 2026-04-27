#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        MeshNodeFSM, MsgPayload, Output, PeerID, RelayPayload,
        test::{drive_bootstrap_handshake, establish_relay_connection, join_mesh},
    };

    #[test]
    fn available_immediately_for_two_peers() {
        let mut alice = MeshNodeFSM::new();
        let mut bob = MeshNodeFSM::new();

        let (host_out, joiner_out) = drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        assert!(host_out.iter().any(|o| matches!(o, Output::Available)));
        assert!(joiner_out.iter().any(|o| matches!(o, Output::Available)));
    }

    #[test]
    fn joiner_available_after_bootstrap_and_after_relay() {
        let mut alice = MeshNodeFSM::new();
        let alice_id = alice.id().clone();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        let mut charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();

        let (bob_bootstrap_out, charlie_bootstrap_out) =
            drive_bootstrap_handshake::<()>(&mut bob, &mut charlie);

        assert!(
            charlie_bootstrap_out
                .iter()
                .any(|o| matches!(o, Output::Available)),
            "charlie should be available immediately after bootstrap"
        );

        let relay_messages: Vec<_> = bob_bootstrap_out
            .into_iter()
            .filter_map(|o| match o {
                Output::SendMessage { peer_to, data } => Some((bob_id.clone(), peer_to, data)),
                _ => None,
            })
            .collect();

        let mut peers = HashMap::new();
        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);
        peers.insert(charlie_id.clone(), charlie);

        let relay_out = establish_relay_connection(
            &mut peers,
            &bob_id,
            &charlie_id,
            &alice_id,
            &relay_messages,
        );

        assert!(
            relay_out.iter().any(|o| matches!(o, Output::Available)),
            "charlie should be available after relay with alice completes"
        );
    }

    #[test]
    fn joiner_waits_for_all_relays() {
        let mut peers: HashMap<PeerID, MeshNodeFSM> = HashMap::new();

        let mut alice = MeshNodeFSM::new();
        let alice_id = alice.id().clone();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();
        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);
        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);

        let charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();
        peers.insert(charlie_id.clone(), charlie);
        join_mesh(&charlie_id, &bob_id, &mut peers);

        let mut alice = peers.remove(&alice_id).unwrap();
        let mut dave = MeshNodeFSM::new();
        let dave_id = dave.id().clone();

        let (alice_bootstrap_out, dave_bootstrap_out) =
            drive_bootstrap_handshake::<()>(&mut alice, &mut dave);

        assert!(
            dave_bootstrap_out
                .iter()
                .any(|o| matches!(o, Output::Available)),
            "dave should be available immediately after bootstrap"
        );

        let appeared: Vec<PeerID> = alice_bootstrap_out
            .iter()
            .filter_map(|o| match o {
                Output::SendMessage {
                    peer_to,
                    data:
                        MsgPayload::RelaySignalingFrom {
                            data: RelayPayload::InitHost(_),
                            ..
                        },
                } => Some(peer_to.clone()),
                _ => None,
            })
            .collect();

        let relay_messages: Vec<_> = alice_bootstrap_out
            .iter()
            .filter_map(|o| match o {
                Output::SendMessage { peer_to, data } => {
                    Some((alice_id.clone(), peer_to.clone(), data.clone()))
                }
                _ => None,
            })
            .collect();

        peers.insert(alice_id.clone(), alice);
        peers.insert(dave_id.clone(), dave);

        assert_eq!(
            appeared.len(),
            2,
            "alice should introduce dave to 2 existing peers"
        );

        let relay1_out = establish_relay_connection(
            &mut peers,
            &alice_id,
            &dave_id,
            &appeared[0],
            &relay_messages,
        );
        assert!(
            relay1_out.iter().any(|o| matches!(o, Output::Available)),
            "dave should be available after first relay completes"
        );

        let relay2_out = establish_relay_connection(
            &mut peers,
            &alice_id,
            &dave_id,
            &appeared[1],
            &relay_messages,
        );
        assert!(
            relay2_out.iter().any(|o| matches!(o, Output::Available)),
            "dave should be available after all relays complete"
        );
    }
}
