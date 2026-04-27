use std::time::Duration;

use antenna_integration_tests::{TestEvent, TestMsg, TestPeer, assert_full_mesh, bridge};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn relay_reconnect_after_forced_drop() {
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
    assert_full_mesh(&[&a, &b, &c]).await;

    a.peer.force_drop(b_id.clone()).await.expect("force_drop");

    let saw_drop_pred = move |id: String| {
        move |e: &TestEvent| {
            matches!(
                e,
                TestEvent::PeerDropped(p) | TestEvent::PeerDisconnected(p) if p.as_str() == id
            )
        }
    };
    a.wait_for(Duration::from_secs(15), saw_drop_pred(b_id.to_string()))
        .await
        .expect("A observes B drop");
    b.wait_for(Duration::from_secs(15), saw_drop_pred(a_id.to_string()))
        .await
        .expect("B observes A drop");

    // Reconnect handshake can take longer than the default timeout: the new
    // peer connection has to redo full ICE + DTLS + SCTP after webrtc-rs releases
    // ports from the just-closed connection. Use a generous deadline.
    let pc_pred = |id: String| {
        move |e: &TestEvent| matches!(e, TestEvent::PeerConnected(p) if p.as_str() == id)
    };
    a.wait_for(Duration::from_secs(60), pc_pred(b_id.to_string()))
        .await
        .expect("A reconnected to B via relay");
    b.wait_for(Duration::from_secs(60), pc_pred(a_id.to_string()))
        .await
        .expect("B reconnected to A via relay");

    assert_full_mesh(&[&a, &b, &c]).await;

    a.peer.send(b_id.clone(), TestMsg("recovered".into()));
    let (from, msg) = b.wait_message().await.unwrap();
    assert_eq!(from, a_id);
    assert_eq!(msg, TestMsg("recovered".into()));

    a.peer.leave();
    b.peer.leave();
    c.peer.leave();
}
