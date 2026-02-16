use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_peer_sends_message() {
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

    // Wait for join event first
    behavior.wait_for_events(1, 5000).await;

    // Send a message
    let test_message = b"Hello, Room!";
    client
        .send_message(test_message)
        .await
        .expect("Failed to send message");

    // Wait for message event
    let received = behavior.wait_for_events(2, 5000).await;
    assert!(received, "Expected at least 2 events (join + message)");

    // Verify message content
    let messages = behavior.messages_from(&peer_id).await;
    assert!(
        !messages.is_empty(),
        "Should have received at least one message"
    );
    assert_eq!(
        messages[0].as_ref(),
        test_message,
        "Message content should match"
    );

    // Cleanup
    client.close().await.expect("Failed to close client");
}
