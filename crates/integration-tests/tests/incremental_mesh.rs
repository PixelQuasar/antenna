use antenna_integration_tests::{TestPeer, assert_full_mesh, bridge};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn five_peer_incremental_mesh() {
    let mut a = TestPeer::new().await;
    let mut b = TestPeer::new().await;
    let mut c = TestPeer::new().await;
    let mut d = TestPeer::new().await;
    let mut e = TestPeer::new().await;
    let a_id = a.id().await;
    let b_id = b.id().await;
    let c_id = c.id().await;
    let d_id = d.id().await;
    let e_id = e.id().await;

    bridge(&a, &b).await.expect("A<->B");
    a.wait_peer_connected(b_id.clone()).await.unwrap();
    b.wait_peer_connected(a_id.clone()).await.unwrap();

    bridge(&b, &c).await.expect("B<->C");
    b.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(b_id.clone()).await.unwrap();
    a.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(a_id.clone()).await.unwrap();

    bridge(&a, &d).await.expect("A<->D");
    a.wait_peer_connected(d_id.clone()).await.unwrap();
    d.wait_peer_connected(a_id.clone()).await.unwrap();
    b.wait_peer_connected(d_id.clone()).await.unwrap();
    d.wait_peer_connected(b_id.clone()).await.unwrap();
    c.wait_peer_connected(d_id.clone()).await.unwrap();
    d.wait_peer_connected(c_id.clone()).await.unwrap();

    bridge(&c, &e).await.expect("C<->E");
    c.wait_peer_connected(e_id.clone()).await.unwrap();
    e.wait_peer_connected(c_id.clone()).await.unwrap();
    a.wait_peer_connected(e_id.clone()).await.unwrap();
    e.wait_peer_connected(a_id.clone()).await.unwrap();
    b.wait_peer_connected(e_id.clone()).await.unwrap();
    e.wait_peer_connected(b_id.clone()).await.unwrap();
    d.wait_peer_connected(e_id.clone()).await.unwrap();
    e.wait_peer_connected(d_id.clone()).await.unwrap();

    assert_full_mesh(&[&a, &b, &c, &d, &e]).await;

    a.peer.leave();
    b.peer.leave();
    c.peer.leave();
    d.peer.leave();
    e.peer.leave();
}
