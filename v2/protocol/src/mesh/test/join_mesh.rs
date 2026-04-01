use std::collections::HashMap;

use crate::{
    Input, MeshNodeFSM, Output, PeerID, RelayPayload, HandshakeInput, HandshakeOutput,
    assert_handshake_event, extract_relay,
    mesh::test::drive_bootstrap_handshake::drive_bootstrap_handshake, relay_through,
};

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
