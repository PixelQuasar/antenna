use std::time::Duration;

use antenna_integration_tests::{TestEvent, TestMsg, TestPeer, bridge};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graceful_leave_in_three_peer_mesh() {
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
    a.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(a_id.clone()).await.unwrap();
    b.wait_peer_connected(c_id.clone()).await.unwrap();
    c.wait_peer_connected(b_id.clone()).await.unwrap();

    c.peer.leave();

    let dropped_predicate = |id: &str| {
        let id = id.to_string();
        move |e: &TestEvent| matches!(e, TestEvent::PeerDisconnected(p) | TestEvent::PeerDropped(p) if p.as_str() == id)
    };
    a.wait_for(Duration::from_secs(60), dropped_predicate(c_id.as_str()))
        .await
        .expect("A sees C depart");
    b.wait_for(Duration::from_secs(60), dropped_predicate(c_id.as_str()))
        .await
        .expect("B sees C depart");

    assert!(a.peer.is_connected(&b_id).await);
    assert!(b.peer.is_connected(&a_id).await);

    a.peer.send(b_id.clone(), TestMsg("still here".into()));
    let (from, msg) = b.wait_message().await.unwrap();
    assert_eq!(from, a_id);
    assert_eq!(msg, TestMsg("still here".into()));

    a.peer.leave();
    b.peer.leave();
}
