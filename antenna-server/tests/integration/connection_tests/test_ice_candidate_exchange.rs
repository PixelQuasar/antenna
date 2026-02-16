use std::time::Duration;

use antenna_core::PeerId;
use antenna_server::RoomCommand;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{SignalMessage, TestClient, TestClientConfig, wait_for_client_ready};

#[tokio::test]
async fn test_ice_candidate_exchange() {
    init_tracing();

    let (room_cmd_tx, mut signal_rx, behavior) = create_test_room();

    let peer_id = PeerId::new();
    let client = TestClient::new(peer_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create test client");

    // Create offer
    let offer = client.create_offer().await.expect("Failed to create offer");

    // Send join request
    room_cmd_tx
        .send(RoomCommand::JoinRequest {
            peer_id: peer_id.clone(),
            offer,
        })
        .await
        .expect("Failed to send join request");

    // Collect ICE candidates from client
    let client_candidates = client
        .gather_ice_candidates(3000)
        .await
        .expect("Failed to gather ICE candidates");

    tracing::info!(
        "Client generated {} ICE candidates",
        client_candidates.len()
    );

    // Send client's ICE candidates to room
    for candidate in client_candidates {
        room_cmd_tx
            .send(RoomCommand::IceCandidate {
                peer_id: peer_id.clone(),
                candidate,
            })
            .await
            .expect("Failed to send ICE candidate");
    }

    // Wait for answer from room
    let mut answer_received = false;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Ok(Some(msg)) =
            tokio::time::timeout(Duration::from_millis(100), signal_rx.recv()).await
        {
            match msg {
                SignalMessage::Answer { peer_id: id, sdp } if id == peer_id => {
                    client
                        .set_remote_answer(sdp)
                        .await
                        .expect("Failed to set answer");
                    answer_received = true;
                }
                SignalMessage::Ice {
                    peer_id: id,
                    candidate,
                } if id == peer_id => {
                    let _ = client.add_ice_candidate(candidate).await;
                }
                _ => {}
            }
        }

        if answer_received {
            // Continue processing ICE candidates for a bit
            tokio::time::sleep(Duration::from_millis(500)).await;
            break;
        }
    }

    assert!(answer_received, "Should have received answer");

    // Wait for connection
    wait_for_client_ready(&client)
        .await
        .expect("Client not ready after ICE exchange");

    // Verify join was called
    behavior.wait_for_events(1, 5000).await;
    assert!(behavior.has_join(&peer_id).await);

    client.close().await.expect("Failed to close client");
}
