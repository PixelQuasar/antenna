use std::cell::RefCell;
use std::collections::HashMap;

use crate::{
    HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, Output, PeerID, RelayPayload,
    assert_handshake_event, extract_relay,
    mesh::test::drive_bootstrap_handshake::drive_bootstrap_handshake, relay_through,
};

thread_local! {
    static CONNECTION_COUNTER: RefCell<usize> = RefCell::new(0);
}

pub(crate) fn reset_connection_counter() {
    CONNECTION_COUNTER.with(|c| *c.borrow_mut() = 0);
}

pub(crate) fn get_connection_count() -> usize {
    CONNECTION_COUNTER.with(|c| *c.borrow())
}

fn max_edges_in_complete_graph(n: usize) -> usize {
    n * (n - 1) / 2
}

pub(crate) fn assert_connection_count_within_limit(peer_count: usize) {
    let actual = get_connection_count();
    let max = max_edges_in_complete_graph(peer_count);

    assert!(
        actual <= max,
        "Too many connections established: {} > {} (max for {} nodes in complete graph)",
        actual,
        max,
        peer_count
    );
}

pub(crate) fn join_mesh(
    new_peer_id: &PeerID,
    bootstrap_id: &PeerID,
    all_peers: &mut HashMap<PeerID, MeshNodeFSM>,
) {
    let connection_requests = {
        let mut bootstrap = all_peers.remove(bootstrap_id).unwrap();
        let mut new_peer = all_peers.remove(new_peer_id).unwrap();

        let outputs = drive_bootstrap_handshake::<()>(&mut bootstrap, &mut new_peer);

        all_peers.insert(bootstrap_id.clone(), bootstrap);
        all_peers.insert(new_peer_id.clone(), new_peer);

        outputs
            .iter()
            .filter_map(|o| match o {
                Output::Relay { via, payload } if via == new_peer_id => match payload {
                    RelayPayload::ConnectionRequest { peer } => Some(peer.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    for existing_peer_id in connection_requests {
        establish_relay_connection(all_peers, new_peer_id, &existing_peer_id, bootstrap_id);
    }
}

fn establish_relay_connection(
    peers: &mut HashMap<PeerID, MeshNodeFSM>,
    initiator_id: &PeerID,
    target_id: &PeerID,
    relay_id: &PeerID,
) {
    CONNECTION_COUNTER.with(|c| *c.borrow_mut() += 1);

    let outputs = peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Relay {
            from: relay_id.clone(),
            payload: RelayPayload::ConnectionRequest {
                peer: target_id.clone(),
            },
        });

    assert_handshake_event!(
        outputs,
        peer: target_id.clone(),
        event: HandshakeOutput::InitSDPOffer
    );

    let outputs = peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: target_id.clone(),
            event: HandshakeInput::SDPOfferCreated {
                sdp: "offer".into(),
            },
        });

    let relay_offer = extract_relay!(outputs, via: relay_id.clone());

    let outputs = relay_through!(
        peers.get_mut(relay_id).unwrap(),
        from: initiator_id.clone(),
        payload: relay_offer
    );
    let relay_to_target = extract_relay!(outputs, via: target_id.clone());

    let outputs = relay_through!(
        peers.get_mut(target_id).unwrap(),
        from: relay_id.clone(),
        payload: relay_to_target
    );

    assert_handshake_event!(
        outputs,
        peer: initiator_id.clone(),
        event: HandshakeOutput::InitSDPAnswer { .. }
    );

    let outputs = peers
        .get_mut(target_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: initiator_id.clone(),
            event: HandshakeInput::SDPAnswerCreated {
                sdp: "answer".into(),
            },
        });

    let relay_answer = extract_relay!(outputs, via: relay_id.clone());

    let outputs = relay_through!(
        peers.get_mut(relay_id).unwrap(),
        from: target_id.clone(),
        payload: relay_answer
    );
    let relay_to_initiator = extract_relay!(outputs, via: initiator_id.clone());

    relay_through!(
        peers.get_mut(initiator_id).unwrap(),
        from: relay_id.clone(),
        payload: relay_to_initiator
    );

    peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: target_id.clone(),
            event: HandshakeInput::DataChannelOpen,
        });

    peers
        .get_mut(target_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: initiator_id.clone(),
            event: HandshakeInput::DataChannelOpen,
        });
}

pub(crate) fn assert_full_mesh_connectivity(peers: &HashMap<PeerID, MeshNodeFSM>) {
    let n = peers.len();

    for (peer_id, mesh) in peers {
        assert_eq!(
            mesh.connected_peers().len(),
            n - 1,
            "Peer {:?} should have {} connections, but has {}",
            peer_id,
            n - 1,
            mesh.connected_peers().len()
        );

        for (other_id, _) in peers {
            if other_id != peer_id {
                assert!(
                    mesh.is_connected(other_id),
                    "Peer {:?} not connected to {:?}",
                    peer_id,
                    other_id
                );
            }
        }
    }
}
