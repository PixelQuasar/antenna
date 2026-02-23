use antenna_core::PeerId;

use crate::integration::{create_test_room, init_tracing};
use crate::utils::{TestClient, TestClientConfig, perform_signaling, wait_for_client_ready};

#[tokio::test]
async fn test_multiple_peers_join() {
    init_tracing();

    let (room_cmd_tx, signaling, behavior) = create_test_room();
    let mut signal_rx = signaling.1;
    let signaling = signaling.0;

    let peer1_id = PeerId::new();
    let peer2_id = PeerId::new();

    let client1 = TestClient::new(peer1_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 1");
    signaling.register_peer(peer1_id.clone());

    let client2 = TestClient::new(peer2_id.clone(), TestClientConfig::default())
        .await
        .expect("Failed to create client 2");
    signaling.register_peer(peer2_id.clone());

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

    assert!(
        behavior.has_join(&peer1_id).await,
        "Peer 1 should have joined"
    );
    assert!(
        behavior.has_join(&peer2_id).await,
        "Peer 2 should have joined"
    );

    client1.close().await.expect("Failed to close client 1");
    client2.close().await.expect("Failed to close client 2");
}
