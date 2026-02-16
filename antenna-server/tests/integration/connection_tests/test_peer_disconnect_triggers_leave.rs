use antenna_core::PeerId;
use antenna_server::RoomCommand;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_peer_disconnect_triggers_leave() {
    init_tracing();

    let (room_cmd_tx, mut signal_rx, behavior) = create_test_room();

    // Create and connect client
    let peer_id = PeerId::new();
    let client = TestClient::new(peer_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create test client");

    perform_signaling(&client, &room_cmd_tx, &mut signal_rx)
        .await
        .expect("Signaling failed");

    wait_for_client_ready(&client)
        .await
        .expect("Client not ready");

    // Wait for join
    behavior.wait_for_events(1, 5000).await;
    assert!(behavior.has_join(&peer_id).await);

    // Send disconnect command
    room_cmd_tx
        .send(RoomCommand::Disconnect {
            peer_id: peer_id.clone(),
        })
        .await
        .expect("Failed to send disconnect");

    // Wait for leave event
    let left = behavior.wait_for_events(2, 5000).await;
    assert!(left, "Expected leave event");
    assert!(
        behavior.has_leave(&peer_id).await,
        "on_leave should have been called"
    );

    // Cleanup
    client.close().await.expect("Failed to close client");
}
