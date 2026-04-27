use antenna_integration_tests::{TestMsg, TestPeer, assert_full_mesh, bridge};

#[tokio::test]
async fn bootstrap_handshake_and_bidirectional_messaging() {
    let mut a = TestPeer::new().await;
    let mut b = TestPeer::new().await;
    let a_id = a.id().await;
    let b_id = b.id().await;

    bridge(&a, &b).await.expect("bridge SDP exchange");
    a.wait_peer_connected(b_id.clone()).await.unwrap();
    b.wait_peer_connected(a_id.clone()).await.unwrap();

    assert_full_mesh(&[&a, &b]).await;

    a.peer.send(b_id.clone(), TestMsg("hello from A".into()));
    let (from, msg) = b.wait_message().await.unwrap();
    assert_eq!(from, a_id);
    assert_eq!(msg, TestMsg("hello from A".into()));

    b.peer.send(a_id.clone(), TestMsg("hello from B".into()));
    let (from, msg) = a.wait_message().await.unwrap();
    assert_eq!(from, b_id);
    assert_eq!(msg, TestMsg("hello from B".into()));

    a.peer.leave();
    b.peer.leave();
}
