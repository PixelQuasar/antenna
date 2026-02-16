use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_peer_leaves_others_stay() {
    init_tracing();

    let (room_cmd_tx, mut signal_rx, behavior) = create_test_room();

    let peer1_id = PeerId::new();
    let peer2_id = PeerId::new();

    let client1 = TestClient::new(peer1_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 1");

    let client2 = TestClient::new(peer2_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 2");

    perform_signaling(&client1, &room_cmd_tx, &mut signal_rx)
        .await
        .expect("Signaling failed for client 1");
    wait_for_client_ready(&client1)
        .await
        .expect("Client 1 not ready");

    perform_signaling(&client2, &room_cmd_tx, &mut signal_rx)
        .await
        .expect("Signaling failed for client 2");
    wait_for_client_ready(&client2)
        .await
        .expect("Client 2 not ready");

    behavior.wait_for_events(2, 5000).await;

    client1
        .send_message(b"Message before leave")
        .await
        .expect("Send failed");
    behavior.wait_for_events(3, 5000).await;

    room_cmd_tx
        .send(antenna_server::RoomCommand::Disconnect {
            peer_id: peer1_id.clone(),
        })
        .await
        .expect("Disconnect failed");

    behavior.wait_for_events(4, 5000).await;
    assert!(
        behavior.has_leave(&peer1_id).await,
        "Peer 1 should have left"
    );

    client2
        .send_message(b"Message after peer 1 left")
        .await
        .expect("Send failed");
    behavior.wait_for_events(5, 5000).await;

    let peer2_messages = behavior.messages_from(&peer2_id).await;
    assert_eq!(peer2_messages.len(), 1, "Peer 2 should have sent 1 message");

    client1.close().await.expect("Failed to close client 1");
    client2.close().await.expect("Failed to close client 2");
}
