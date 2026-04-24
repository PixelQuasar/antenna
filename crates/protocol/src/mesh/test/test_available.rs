#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        MeshNodeFSM, Output, PeerID,
        test::{drive_bootstrap_handshake, establish_relay_connection, join_mesh},
    };

    #[test]
    fn available_immediately_for_two_peers() {
        let mut alice = MeshNodeFSM::new();
        let mut bob = MeshNodeFSM::new();

        assert!(!alice.is_available());
        assert!(!bob.is_available());

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        assert!(alice.is_available());
        assert!(bob.is_available());
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

        let bootstrap_outputs = drive_bootstrap_handshake::<()>(&mut bob, &mut charlie);

        assert!(
            charlie.is_available(),
            "charlie should be available immediately after bootstrap"
        );

        let relay_messages: Vec<_> = bootstrap_outputs
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

        establish_relay_connection(&mut peers, &bob_id, &charlie_id, &alice_id, &relay_messages);

        assert!(
            peers[&charlie_id].is_available(),
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

        let mut charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();
        peers.insert(charlie_id.clone(), charlie);
        join_mesh(&charlie_id, &bob_id, &mut peers);

        let mut alice = peers.remove(&alice_id).unwrap();
        let mut dave = MeshNodeFSM::new();
        let dave_id = dave.id().clone();

        let bootstrap_outputs = drive_bootstrap_handshake::<()>(&mut alice, &mut dave);

        assert!(
            dave.is_available(),
            "dave should be available immediately after bootstrap"
        );

        let appeared: Vec<PeerID> = bootstrap_outputs
            .iter()
            .filter_map(|o| match o {
                Output::PeerAppeared { peer } => Some(peer.clone()),
                _ => None,
            })
            .collect();

        let relay_messages: Vec<_> = bootstrap_outputs
            .into_iter()
            .filter_map(|o| match o {
                Output::SendMessage { peer_to, data } => Some((alice_id.clone(), peer_to, data)),
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

        establish_relay_connection(
            &mut peers,
            &alice_id,
            &dave_id,
            &appeared[0],
            &relay_messages,
        );
        assert!(
            peers[&dave_id].is_available(),
            "dave should be available after first relay completes"
        );

        establish_relay_connection(
            &mut peers,
            &alice_id,
            &dave_id,
            &appeared[1],
            &relay_messages,
        );
        assert!(
            peers[&dave_id].is_available(),
            "dave should be available after all relays complete"
        );
    }
}
