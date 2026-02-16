use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_single_peer_joins_room() {
    init_tracing();

    let (room_cmd_tx, mut signal_rx, behavior) = create_test_room();

    // Create test client
    let peer_id = PeerId::new();
    let client = TestClient::new(peer_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create test client");

    // Perform signaling
    perform_signaling(&client, &room_cmd_tx, &mut signal_rx)
        .await
        .expect("Signaling failed");

    // Wait for connection to be established
    wait_for_client_ready(&client)
        .await
        .expect("Client not ready");

    // Verify on_join was called
    let joined = behavior.wait_for_events(1, 5000).await;
    assert!(joined, "Expected at least 1 event");
    assert!(
        behavior.has_join(&peer_id).await,
        "on_join should have been called"
    );

    // Cleanup
    client.close().await.expect("Failed to close client");
}
