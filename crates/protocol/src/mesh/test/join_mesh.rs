use std::cell::RefCell;
use std::collections::HashMap;

use crate::{
    HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, Output, PeerID, assert_handshake_event,
    mesh::test::drive_bootstrap_handshake::drive_bootstrap_handshake,
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
    let appeared_peers = {
        let mut bootstrap = all_peers.remove(bootstrap_id).unwrap();
        let mut new_peer = all_peers.remove(new_peer_id).unwrap();

        let outputs = drive_bootstrap_handshake::<()>(&mut bootstrap, &mut new_peer);

        all_peers.insert(bootstrap_id.clone(), bootstrap);
        all_peers.insert(new_peer_id.clone(), new_peer);

        outputs
            .iter()
            .filter_map(|o| match o {
                Output::PeerAppeared { peer } => Some(peer.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    for existing_peer_id in appeared_peers {
        establish_direct_connection(all_peers, new_peer_id, &existing_peer_id);
    }
}

/// Drives a full direct handshake between two peers that are not yet connected.
/// The initiator acts as host, the target acts as joiner.
fn establish_direct_connection(
    peers: &mut HashMap<PeerID, MeshNodeFSM>,
    initiator_id: &PeerID,
    target_id: &PeerID,
) {
    CONNECTION_COUNTER.with(|c| *c.borrow_mut() += 1);

    // Initiator starts handshake as host
    let outputs = peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: target_id.clone(),
            event: HandshakeInput::InitNegotiation,
        });

    assert_handshake_event!(
        outputs,
        peer: target_id.clone(),
        event: HandshakeOutput::InitSDPOffer
    );

    // Initiator creates SDP offer
    peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: target_id.clone(),
            event: HandshakeInput::SDPOfferCreated {
                sdp: "offer".into(),
            },
        });

    // Target receives SDP offer
    let outputs = peers
        .get_mut(target_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: initiator_id.clone(),
            event: HandshakeInput::SDPOfferReceived {
                sdp: "offer".into(),
            },
        });

    assert_handshake_event!(
        outputs,
        peer: initiator_id.clone(),
        event: HandshakeOutput::RequestSDPAnswer { .. }
    );

    // Target creates SDP answer
    peers
        .get_mut(target_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: initiator_id.clone(),
            event: HandshakeInput::SDPAnswerCreated {
                sdp: "answer".into(),
            },
        });

    // Initiator receives SDP answer
    let outputs = peers
        .get_mut(initiator_id)
        .unwrap()
        .process::<()>(Input::Handshake {
            from: target_id.clone(),
            event: HandshakeInput::SDPAnswerReceived {
                sdp: "answer".into(),
            },
        });

    assert_handshake_event!(
        outputs,
        peer: target_id.clone(),
        event: HandshakeOutput::AcceptSDPAnswer { .. }
    );

    // Both sides open data channel
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
