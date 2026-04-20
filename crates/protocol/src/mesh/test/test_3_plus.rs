#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        MeshNodeFSM, PeerID,
        mesh::test::{
            drive_bootstrap_handshake::drive_bootstrap_handshake,
            join_mesh::{assert_full_mesh_connectivity, join_mesh},
        },
    };

    #[test]
    fn three_peer_mesh() {
        let mut alice = MeshNodeFSM::new();
        let alice_id = alice.id().clone();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();

        let mut peers: HashMap<PeerID, MeshNodeFSM> = HashMap::new();

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 2);

        let charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();
        peers.insert(charlie_id.clone(), charlie);

        join_mesh(&charlie_id, &bob_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 3);

        for mesh in peers.values() {
            assert_eq!(mesh.connected_number(), 2);
        }
    }

    #[test]
    fn incremental() {
        let mut alice = MeshNodeFSM::new();
        let alice_id = alice.id().clone();
        let mut bob = MeshNodeFSM::new();
        let bob_id = bob.id().clone();

        let mut peers: HashMap<PeerID, MeshNodeFSM> = HashMap::new();

        drive_bootstrap_handshake::<()>(&mut alice, &mut bob);

        peers.insert(alice_id.clone(), alice);
        peers.insert(bob_id.clone(), bob);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 2);

        let charlie = MeshNodeFSM::new();
        let charlie_id = charlie.id().clone();
        peers.insert(charlie_id.clone(), charlie);

        join_mesh(&charlie_id, &bob_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 3);

        let dave = MeshNodeFSM::new();
        let dave_id = dave.id().clone();
        peers.insert(dave_id.clone(), dave);

        join_mesh(&dave_id, &alice_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 4);

        let eve = MeshNodeFSM::new();
        let eve_id = eve.id().clone();
        peers.insert(eve_id.clone(), eve);

        join_mesh(&eve_id, &charlie_id, &mut peers);

        assert_full_mesh_connectivity(&peers);
        assert_eq!(peers.len(), 5);

        for mesh in peers.values() {
            assert_eq!(mesh.connected_number(), 4);
        }
    }
}
