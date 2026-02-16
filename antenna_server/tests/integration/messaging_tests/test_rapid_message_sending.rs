use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_rapid_message_sending() {
    init_tracing();

    let (room_cmd_tx, mut signal_rx, behavior) = create_test_room();

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

    behavior.wait_for_events(1, 5000).await;

    // Send many messages rapidly
    let message_count = 10;
    for i in 0..message_count {
        let msg = format!("Rapid message {}", i);
        client
            .send_message(msg.as_bytes())
            .await
            .expect("Failed to send message");
    }

    // Wait for all messages
    let expected_events = 1 + message_count; // join + messages
    behavior.wait_for_events(expected_events, 10000).await;

    let messages = behavior.messages_from(&peer_id).await;
    assert_eq!(
        messages.len(),
        message_count,
        "Should have received all {} messages",
        message_count
    );

    // Verify order
    for (i, msg) in messages.iter().enumerate() {
        let expected = format!("Rapid message {}", i);
        assert_eq!(
            msg.as_ref(),
            expected.as_bytes(),
            "Message {} should match",
            i
        );
    }

    client.close().await.expect("Failed to close client");
}
