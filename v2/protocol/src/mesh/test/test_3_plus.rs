#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        MeshFSM, PeerID,
        mesh::test::{
            drive_bootstrap_handshake::drive_bootstrap_handshake,
            join_mesh::{assert_full_mesh_connectivity, join_mesh},
        },
    };

    #[test]
    fn three_peer_mesh() {
        let alice_id = PeerID::new("alice");
        let bob_id = PeerID::new("bob");
        let charlie_id = PeerID::new("charlie");

        let mut peers = HashMap::new();

        let mut alice = MeshFSM::new(alice_id.clone());
        let mut bob = MeshFSM::new(bob_id.clone());

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 2);

        let charlie = MeshFSM::new(charlie_id.clone());
        peers.insert(charlie_id.clone(), charlie);

        join_mesh(&charlie_id, &bob_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 3);

        for mesh in peers.values() {
            assert_eq!(mesh.connected_peers().len(), 2);
        }
    }

    #[test]
    fn incremental() {
        let alice_id = PeerID::new("alice");
        let bob_id = PeerID::new("bob");
        let charlie_id = PeerID::new("charlie");
        let dave_id = PeerID::new("dave");
        let eve_id = PeerID::new("eve");

        let mut peers = HashMap::new();

        let mut alice = MeshFSM::new(alice_id.clone());
        let mut bob = MeshFSM::new(bob_id.clone());

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 2);

        let charlie = MeshFSM::new(charlie_id.clone());
        peers.insert(charlie_id.clone(), charlie);

        join_mesh(&charlie_id, &bob_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 3);

        let dave = MeshFSM::new(dave_id.clone());
        peers.insert(dave_id.clone(), dave);

        join_mesh(&dave_id, &alice_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 4);

        let eve = MeshFSM::new(eve_id.clone());
        peers.insert(eve_id.clone(), eve);

        join_mesh(&eve_id, &charlie_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 5);

        for mesh in peers.values() {
            assert_eq!(mesh.connected_peers().len(), 4);
        }
    }
}
