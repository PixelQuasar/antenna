use antenna_integration_tests::{TestMsg, TestPeer, assert_full_mesh, bridge};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_peer_mesh_via_relay() {
    let mut a = TestPeer::new().await;
    let mut b = TestPeer::new().await;
    let mut c = TestPeer::new().await;
    let a_id = a.id().await;
    let b_id = b.id().await;
    let c_id = c.id().await;

    bridge(&a, &b).await.expect("bridge A<->B");
    a.wait_peer_connected(b_id.clone()).await.unwrap();
    b.wait_peer_connected(a_id.clone()).await.unwrap();

    bridge(&b, &c).await.expect("bridge B<->C");
    b.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(b_id.clone()).await.unwrap();
    a.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(a_id.clone()).await.unwrap();

    assert_full_mesh(&[&a, &b, &c]).await;

    a.peer.send(c_id.clone(), TestMsg("A->C".into()));
    let (from, msg) = c.wait_message().await.unwrap();
    assert_eq!(from, a_id);
    assert_eq!(msg, TestMsg("A->C".into()));

    c.peer.send(a_id.clone(), TestMsg("C->A".into()));
    let (from, msg) = a.wait_message().await.unwrap();
    assert_eq!(from, c_id);
    assert_eq!(msg, TestMsg("C->A".into()));

    a.peer.leave();
    b.peer.leave();
    c.peer.leave();
}
