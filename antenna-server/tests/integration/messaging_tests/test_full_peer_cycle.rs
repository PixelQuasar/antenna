use std::time::Duration;

use antenna_core::PeerId;
use antenna_server::RoomCommand;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{
    RoomEvent, TestClient, TestClientConfig, perform_signaling, wait_for_client_ready,
};

#[tokio::test]
async fn test_full_peer_lifecycle() {
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
    assert!(behavior.has_join(&peer_id).await, "Should have joined");

    for i in 0..3 {
        let msg = format!("Message {}", i);
        client
            .send_message(msg.as_bytes())
            .await
            .expect("Send failed");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    behavior.wait_for_events(4, 5000).await;

    let messages = behavior.messages_from(&peer_id).await;
    assert_eq!(messages.len(), 3, "Should have received 3 messages");

    room_cmd_tx
        .send(RoomCommand::Disconnect {
            peer_id: peer_id.clone(),
        })
        .await
        .expect("Disconnect failed");

    behavior.wait_for_events(5, 5000).await;
    assert!(behavior.has_leave(&peer_id).await, "Should have left");

    let events = behavior.get_events().await;
    assert!(matches!(events.first(), Some(RoomEvent::Join { .. })));
    assert!(matches!(events.last(), Some(RoomEvent::Leave { .. })));

    client.close().await.expect("Close failed");
}
