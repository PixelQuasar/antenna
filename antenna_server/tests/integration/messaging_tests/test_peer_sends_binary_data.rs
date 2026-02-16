use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_peer_sends_binary_data() {
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

    // Send binary data (not valid UTF-8)
    let binary_data: Vec<u8> = (0..255).collect();
    client
        .send_message(&binary_data)
        .await
        .expect("Failed to send binary data");

    // Wait for message
    behavior.wait_for_events(2, 5000).await;

    let messages = behavior.messages_from(&peer_id).await;
    assert_eq!(messages.len(), 1, "Should have received 1 message");
    assert_eq!(
        messages[0].as_ref(),
        binary_data.as_slice(),
        "Binary data should match"
    );

    client.close().await.expect("Failed to close client");
}
