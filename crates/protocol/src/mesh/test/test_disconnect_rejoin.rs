#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        Input, MeshNodeFSM, MsgPayload, Output, PeerID,
        test::{assert_full_mesh_connectivity, drive_bootstrap_handshake, join_mesh},
    };

    fn deliver_disconnect(peer: &mut MeshNodeFSM, from: &PeerID) -> Vec<Output<()>> {
        peer.process::<()>(Input::MessageReceived {
            peer_from: from.clone(),
            data: MsgPayload::Disconnect,
        })
        .unwrap()
    }

    fn build_three_peer_mesh() -> (PeerID, PeerID, PeerID, HashMap<PeerID, MeshNodeFSM>) {
        let mut alice = MeshNodeFSM::new();
        let alice_id = alice.id().clone();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();
        let charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        let mut peers: HashMap<PeerID, MeshNodeFSM> = HashMap::new();
        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);
        peers.insert(charlie_id.clone(), charlie);

        join_mesh(&charlie_id, &bob_id, &mut peers);
        assert_full_mesh_connectivity(&peers);

        (alice_id, bob_id, charlie_id, peers)
    }

    #[test]
    fn peer_leaves_then_rejoins_three_peer_mesh() {
        let (alice_id, bob_id, charlie_id, mut peers) = build_three_peer_mesh();

        let bob_leave_out = peers
            .get_mut(&bob_id)
            .unwrap()
            .process::<()>(Input::Leave)
            .unwrap();

        assert!(
            bob_leave_out
                .iter()
                .any(|o| matches!(o, Output::Disconnecting))
        );
        let disconnect_targets: Vec<PeerID> = bob_leave_out
            .iter()
            .filter_map(|o| match o {
                Output::SendMessage {
                    peer_to,
                    data: MsgPayload::Disconnect,
                } => Some(peer_to.clone()),
                _ => None,
            })
            .collect();
        assert!(disconnect_targets.contains(&alice_id));
        assert!(disconnect_targets.contains(&charlie_id));

        for peer_id in [&alice_id, &charlie_id] {
            let out = deliver_disconnect(peers.get_mut(peer_id).unwrap(), &bob_id);
            assert!(
                out.iter()
                    .any(|o| matches!(o, Output::PeerDisconnected { peer } if peer == &bob_id)),
                "{peer_id:?} should observe PeerDisconnected for bob"
            );
        }
        assert!(!peers.get(&alice_id).unwrap().is_connected(&bob_id));
        assert!(!peers.get(&charlie_id).unwrap().is_connected(&bob_id));

        join_mesh(&bob_id, &alice_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert!(peers.get(&alice_id).unwrap().is_connected(&bob_id));
        assert!(peers.get(&charlie_id).unwrap().is_connected(&bob_id));
    }
}
