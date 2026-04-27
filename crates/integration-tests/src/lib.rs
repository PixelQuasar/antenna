use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::timeout;

use antenna::{
    Event, IceServerConfig, MessageCallback, NoArgCallback, Peer, PeerCallback, PeerID, Storage,
};

/// User payload type for tests — small wrapper around String for easy assertions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestMsg(pub String);

/// Driver-emitted events
#[derive(Debug, Clone)]
pub enum TestEvent {
    Connected,
    Disconnected,
    PeerConnected(PeerID),
    PeerDisconnected(PeerID),
    PeerDropped(PeerID),
    Message(PeerID, TestMsg),
    Available,
    Unavailable,
}

/// Test wrapper bundling a `Peer<TestMsg>`
pub struct TestPeer {
    pub peer: Arc<Peer<TestMsg>>,
    events: mpsc::UnboundedReceiver<TestEvent>,
    pending: Vec<TestEvent>,
    _storage_file: NamedTempFile,
}

impl TestPeer {
    /// Construct a fresh peer with NO ICE servers
    pub async fn new() -> Self {
        Self::with_ice_servers(Vec::new()).await
    }

    pub async fn with_ice_servers(ice_servers: Vec<IceServerConfig>) -> Self {
        let storage_file = NamedTempFile::new().expect("create tmp identity file");
        let path = storage_file.path().to_string_lossy().into_owned();
        let peer = Arc::new(Peer::with_ice_servers(Storage::new(path), ice_servers));

        let (tx, rx) = mpsc::unbounded_channel();

        // Forward each event variant into the unified stream
        {
            let tx = tx.clone();
            peer.subscribe(Event::Connected(NoArgCallback::from_fn(move || {
                let _ = tx.send(TestEvent::Connected);
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::Disconnected(NoArgCallback::from_fn(move || {
                let _ = tx.send(TestEvent::Disconnected);
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::PeerConnected(PeerCallback::from_fn(move |p| {
                let _ = tx.send(TestEvent::PeerConnected(p.clone()));
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::PeerDisconnected(PeerCallback::from_fn(move |p| {
                let _ = tx.send(TestEvent::PeerDisconnected(p.clone()));
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::PeerLost(PeerCallback::from_fn(move |p| {
                let _ = tx.send(TestEvent::PeerDropped(p.clone()));
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::UserMessage(MessageCallback::<TestMsg>::from_fn(
                move |from, msg| {
                    let _ = tx.send(TestEvent::Message(from.clone(), msg.clone()));
                    Ok(())
                },
            )))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::Available(NoArgCallback::from_fn(move || {
                let _ = tx.send(TestEvent::Available);
                Ok(())
            })))
            .await;
        }
        {
            let tx = tx.clone();
            peer.subscribe(Event::Unavailable(NoArgCallback::from_fn(move || {
                let _ = tx.send(TestEvent::Unavailable);
                Ok(())
            })))
            .await;
        }

        Self {
            peer,
            events: rx,
            pending: Vec::new(),
            _storage_file: storage_file,
        }
    }

    pub async fn id(&self) -> PeerID {
        self.peer.my_id().await
    }

    /// Drain events until `pred` returns true, or the timeout expires
    pub async fn wait_for<F>(&mut self, dur: Duration, mut pred: F) -> Result<TestEvent>
    where
        F: FnMut(&TestEvent) -> bool,
    {
        if let Some(idx) = self.pending.iter().position(&mut pred) {
            return Ok(self.pending.remove(idx));
        }

        let fut = async {
            loop {
                let event = self
                    .events
                    .recv()
                    .await
                    .ok_or_else(|| anyhow!("event channel closed"))?;
                if pred(&event) {
                    return Ok::<_, anyhow::Error>(event);
                }
                self.pending.push(event);
            }
        };
        timeout(dur, fut)
            .await
            .map_err(|_| anyhow!("timeout waiting for event"))?
    }

    pub async fn wait_peer_connected(&mut self, expected: PeerID) -> Result<()> {
        let want = expected.clone();
        self.wait_for(
            default_timeout(),
            move |e| matches!(e, TestEvent::PeerConnected(p) if *p == expected),
        )
        .await
        .map_err(|e| anyhow!("waiting for PeerConnected({want}): {e}"))?;
        Ok(())
    }

    pub async fn wait_peer_disconnected(&mut self, expected: PeerID) -> Result<()> {
        let want = expected.clone();
        self.wait_for(
            default_timeout(),
            move |e| matches!(e, TestEvent::PeerDisconnected(p) if *p == expected),
        )
        .await
        .map_err(|e| anyhow!("waiting for PeerDisconnected({want}): {e}"))?;
        Ok(())
    }

    pub async fn wait_peer_dropped(&mut self, expected: PeerID) -> Result<()> {
        let want = expected.clone();
        self.wait_for(
            default_timeout(),
            move |e| matches!(e, TestEvent::PeerDropped(p) if *p == expected),
        )
        .await
        .map_err(|e| anyhow!("waiting for PeerDropped({want}): {e}"))?;
        Ok(())
    }

    pub async fn wait_message(&mut self) -> Result<(PeerID, TestMsg)> {
        let event = self
            .wait_for(default_timeout(), |e| matches!(e, TestEvent::Message(_, _)))
            .await?;
        match event {
            TestEvent::Message(from, msg) => Ok((from, msg)),
            _ => unreachable!("predicate filtered to Message variant"),
        }
    }
}

/// Assert that every peer in the slice is connected to every other peer
pub async fn assert_full_mesh(peers: &[&TestPeer]) {
    let ids: Vec<PeerID> = {
        let mut out = Vec::with_capacity(peers.len());
        for p in peers {
            out.push(p.id().await);
        }
        out
    };
    for (i, peer) in peers.iter().enumerate() {
        let actual = peer.peer.connected_peers().await;
        let expected: std::collections::HashSet<PeerID> = ids
            .iter()
            .enumerate()
            .filter_map(|(j, id)| if i == j { None } else { Some(id.clone()) })
            .collect();
        assert_eq!(
            actual, expected,
            "peer {} connectivity mismatch: have {:?}, want {:?}",
            ids[i], actual, expected
        );
    }
}

pub async fn bridge(host: &TestPeer, joiner: &TestPeer) -> Result<()> {
    let offer = host.peer.start().await?;
    let answer = joiner.peer.receive_offer(&offer).await?;
    host.peer.receive_answer(&answer).await?;
    Ok(())
}

pub fn default_timeout() -> Duration {
    Duration::from_secs(10)
}
