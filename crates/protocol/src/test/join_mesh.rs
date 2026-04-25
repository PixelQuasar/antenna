use std::collections::{HashMap, VecDeque};

use crate::{
    HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, MsgPayload, Output, PeerID,
    test::drive_bootstrap_handshake,
};

pub(crate) fn join_mesh(
    new_peer_id: &PeerID,
    bootstrap_id: &PeerID,
    all_peers: &mut HashMap<PeerID, MeshNodeFSM>,
) {
    let bootstrap_outputs = {
        let mut bootstrap = all_peers.remove(bootstrap_id).unwrap();
        let mut new_peer = all_peers.remove(new_peer_id).unwrap();

        let (host_out, _) = drive_bootstrap_handshake::<()>(&mut bootstrap, &mut new_peer);

        all_peers.insert(bootstrap_id.clone(), bootstrap);
        all_peers.insert(new_peer_id.clone(), new_peer);

        host_out
    };

    let appeared_peers = bootstrap_outputs
        .iter()
        .filter_map(|o| match o {
            Output::PeerAppeared { peer } => Some(peer.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let relay_messages = bootstrap_outputs
        .into_iter()
        .filter_map(|o| match o {
            Output::SendMessage { peer_to, data } => Some((bootstrap_id.clone(), peer_to, data)),
            _ => None,
        })
        .collect::<Vec<_>>();

    for existing_peer_id in appeared_peers {
        establish_relay_connection(
            all_peers,
            bootstrap_id,
            new_peer_id,
            &existing_peer_id,
            &relay_messages,
        );
    }
}

pub(crate) fn establish_relay_connection(
    peers: &mut HashMap<PeerID, MeshNodeFSM>,
    relay_id: &PeerID,
    joiner_id: &PeerID,
    host_id: &PeerID,
    initial_messages: &[(PeerID, PeerID, MsgPayload<()>)],
) -> Vec<Output<()>> {
    let mut queue = VecDeque::new();
    let mut collected: Vec<Output<()>> = vec![];

    for (from, to, data) in initial_messages {
        let is_for_pair = matches!(
            data,
            MsgPayload::RelaySignalingFrom { src, .. }
                if (to == host_id && src == joiner_id) || (to == joiner_id && src == host_id)
        );

        if is_for_pair {
            queue.push_back((from.clone(), to.clone(), data.clone()));
        }
    }

    let mut host_dc_open = false;
    let mut joiner_dc_open = false;

    while let Some((from, to, data)) = queue.pop_front() {
        let outputs = peers
            .get_mut(&to)
            .unwrap()
            .process::<()>(Input::MessageReceived {
                peer_from: from.clone(),
                data,
            })
            .unwrap();

        collected.extend(outputs.iter().cloned());

        for output in outputs {
            match output {
                Output::Handshake { peer, event } => match event {
                    HandshakeOutput::InitSDPOffer => {
                        assert_eq!(to, *host_id);
                        assert_eq!(peer, *joiner_id);

                        let outputs = peers
                            .get_mut(host_id)
                            .unwrap()
                            .process::<()>(Input::Handshake {
                                from: joiner_id.clone(),
                                event: HandshakeInput::OfferCreated("offer".into()),
                            })
                            .unwrap();

                        collected.extend(outputs.iter().cloned());
                        collect_relay_signaling_to(
                            outputs, relay_id, joiner_id, &mut queue, host_id,
                        );
                    }
                    HandshakeOutput::RequestSDPAnswer(_) => {
                        assert_eq!(to, *joiner_id);
                        assert_eq!(peer, *host_id);

                        let outputs = peers
                            .get_mut(joiner_id)
                            .unwrap()
                            .process::<()>(Input::Handshake {
                                from: host_id.clone(),
                                event: HandshakeInput::AnswerCreated("answer".into()),
                            })
                            .unwrap();

                        collected.extend(outputs.iter().cloned());
                        collect_relay_signaling_to(
                            outputs, relay_id, host_id, &mut queue, joiner_id,
                        );
                    }
                    HandshakeOutput::AcceptSDPAnswer(_) => {
                        assert_eq!(to, *host_id);
                        assert_eq!(peer, *joiner_id);

                        collected.extend(
                            peers
                                .get_mut(host_id)
                                .unwrap()
                                .process::<()>(Input::Handshake {
                                    from: joiner_id.clone(),
                                    event: HandshakeInput::DataChannelOpen,
                                })
                                .unwrap(),
                        );
                        host_dc_open = true;

                        collected.extend(
                            peers
                                .get_mut(joiner_id)
                                .unwrap()
                                .process::<()>(Input::Handshake {
                                    from: host_id.clone(),
                                    event: HandshakeInput::DataChannelOpen,
                                })
                                .unwrap(),
                        );
                        joiner_dc_open = true;
                    }
                    HandshakeOutput::Connected => {}
                    other => panic!("unexpected handshake output for relay test: {other:?}"),
                },
                Output::SendMessage { peer_to, data } => {
                    queue.push_back((to.clone(), peer_to, data));
                }
                Output::PeerAppeared { peer } => {
                    assert_eq!(to, *joiner_id);
                    assert_eq!(peer, *host_id);
                }
                Output::PeerConnected { peer } => {
                    if to == *host_id {
                        assert_eq!(peer, *joiner_id);
                    } else if to == *joiner_id {
                        assert_eq!(peer, *host_id);
                    } else {
                        panic!("unexpected peer connected emitter: {to:?}");
                    }
                }
                Output::PeerDisconnected { peer } => {
                    panic!("unexpected disconnect during relay test: {peer:?}");
                }
                Output::ReceiveMessage { .. } => {}
                Output::InitOpenOffer => {}
                Output::Available => {}
                Output::Unavailable => {}
            }
        }
    }

    assert!(host_dc_open, "host side data channel never opened");
    assert!(joiner_dc_open, "joiner side data channel never opened");
    assert!(peers.get(host_id).unwrap().is_connected(joiner_id));
    assert!(peers.get(joiner_id).unwrap().is_connected(host_id));

    collected
}

fn collect_relay_signaling_to(
    outputs: Vec<Output<()>>,
    relay_id: &PeerID,
    dst: &PeerID,
    queue: &mut VecDeque<(PeerID, PeerID, MsgPayload<()>)>,
    sender_id: &PeerID,
) {
    let mut found = false;

    for output in outputs {
        if let Output::SendMessage { peer_to, data } = output {
            if let MsgPayload::RelaySignalingTo {
                dst: actual_dst, ..
            } = &data
            {
                assert_eq!(peer_to, *relay_id, "relay message must go to relay node");
                assert_eq!(*actual_dst, *dst, "relay message must target correct peer");
                queue.push_back((sender_id.clone(), relay_id.clone(), data));
                found = true;
            }
        }
    }

    assert!(found, "expected relay signaling message was not produced");
}

pub(crate) fn assert_full_mesh_connectivity(peers: &HashMap<PeerID, MeshNodeFSM>) {
    let n = peers.len();

    for (peer_id, mesh) in peers {
        assert_eq!(
            mesh.connected_number(),
            n - 1,
            "Peer {:?} should have {} connections, but has {}",
            peer_id,
            n - 1,
            mesh.connected_number()
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
