use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_three_peers_join() {
    init_tracing();

    let (room_cmd_tx, signaling, behavior) = create_test_room();
    let mut signal_rx = signaling.1;
    let signaling = signaling.0;

    let peer1_id = PeerId::new();
    let peer2_id = PeerId::new();
    let peer3_id = PeerId::new();

    let client1 = TestClient::new(peer1_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 1");
    signaling.register_peer(peer1_id.clone());

    let client2 = TestClient::new(peer2_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 2");
    signaling.register_peer(peer2_id.clone());

    let client3 = TestClient::new(peer3_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 3");
    signaling.register_peer(peer3_id.clone());

    for (client, name) in [(&client1, "1"), (&client2, "2"), (&client3, "3")] {
        perform_signaling(client, &room_cmd_tx, &mut signal_rx)
            .await
            .unwrap_or_else(|_| panic!("Signaling failed for client {}", name));
        wait_for_client_ready(client)
            .await
            .unwrap_or_else(|_| panic!("Client {} not ready", name));
    }

    behavior.wait_for_events(3, 5000).await;

    assert!(
        behavior.has_join(&peer1_id).await,
        "Peer 1 should have joined"
    );
    assert!(
        behavior.has_join(&peer2_id).await,
        "Peer 2 should have joined"
    );
    assert!(
        behavior.has_join(&peer3_id).await,
        "Peer 3 should have joined"
    );

    client1
        .send_message(b"Hello from peer 1")
        .await
        .expect("Send failed");
    client2
        .send_message(b"Hello from peer 2")
        .await
        .expect("Send failed");
    client3
        .send_message(b"Hello from peer 3")
        .await
        .expect("Send failed");

    behavior.wait_for_events(6, 5000).await;

    assert_eq!(behavior.messages_from(&peer1_id).await.len(), 1);
    assert_eq!(behavior.messages_from(&peer2_id).await.len(), 1);
    assert_eq!(behavior.messages_from(&peer3_id).await.len(), 1);

    client1.close().await.expect("Failed to close client 1");
    client2.close().await.expect("Failed to close client 2");
    client3.close().await.expect("Failed to close client 3");
}
