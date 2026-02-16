use antenna_core::PeerId;
use antenna_server::{RoomBehavior, RoomContext};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Event types that can be recorded by TestRoomBehavior.
#[derive(Debug, Clone)]
pub enum RoomEvent {
    /// A peer joined the room.
    Join { peer_id: PeerId },
    /// A message was received from a peer.
    Message { peer_id: PeerId, data: Bytes },
    /// A peer left the room.
    Leave { peer_id: PeerId },
}

/// A test implementation of RoomBehavior that records all events.
///
/// # Example
///
/// ```ignore
/// let behavior = TestRoomBehavior::new();
/// let events = behavior.events();
///
/// // ... run room with this behavior ...
///
/// let recorded = events.lock().await;
/// assert!(recorded.iter().any(|e| matches!(e, RoomEvent::Join { .. })));
/// ```
#[derive(Clone)]
pub struct TestRoomBehavior {
    events: Arc<Mutex<Vec<RoomEvent>>>,
    /// Optional callback to execute on join
    on_join_callback: Option<Arc<dyn Fn(&RoomContext, PeerId) + Send + Sync>>,
}

impl TestRoomBehavior {
    /// Create a new TestRoomBehavior with empty event log.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            on_join_callback: None,
        }
    }

    /// Create a new TestRoomBehavior with a custom on_join callback.
    pub fn with_on_join<F>(callback: F) -> Self
    where
        F: Fn(&RoomContext, PeerId) + Send + Sync + 'static,
    {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            on_join_callback: Some(Arc::new(callback)),
        }
    }

    /// Get a clone of the events Arc for verification.
    pub fn events(&self) -> Arc<Mutex<Vec<RoomEvent>>> {
        Arc::clone(&self.events)
    }

    /// Get all recorded events (convenience method).
    pub async fn get_events(&self) -> Vec<RoomEvent> {
        self.events.lock().await.clone()
    }

    /// Wait for a specific number of events with timeout.
    pub async fn wait_for_events(&self, count: usize, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            if self.events.lock().await.len() >= count {
                return true;
            }
            if start.elapsed() > timeout {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// Check if a join event was recorded for the given peer.
    pub async fn has_join(&self, peer_id: &PeerId) -> bool {
        self.events
            .lock()
            .await
            .iter()
            .any(|e| matches!(e, RoomEvent::Join { peer_id: id } if id == peer_id))
    }

    /// Check if a leave event was recorded for the given peer.
    pub async fn has_leave(&self, peer_id: &PeerId) -> bool {
        self.events
            .lock()
            .await
            .iter()
            .any(|e| matches!(e, RoomEvent::Leave { peer_id: id } if id == peer_id))
    }

    /// Get all messages received from a specific peer.
    pub async fn messages_from(&self, peer_id: &PeerId) -> Vec<Bytes> {
        self.events
            .lock()
            .await
            .iter()
            .filter_map(|e| match e {
                RoomEvent::Message { peer_id: id, data } if id == peer_id => Some(data.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Default for TestRoomBehavior {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoomBehavior for TestRoomBehavior {
    async fn on_join(&self, ctx: &RoomContext, peer_id: PeerId) {
        tracing::info!("[TestBehavior] on_join: {:?}", peer_id);

        self.events.lock().await.push(RoomEvent::Join {
            peer_id: peer_id.clone(),
        });

        // Execute optional callback
        if let Some(callback) = &self.on_join_callback {
            callback(ctx, peer_id);
        }
    }

    async fn on_message(&self, _ctx: &RoomContext, peer_id: PeerId, data: Bytes) {
        tracing::info!(
            "[TestBehavior] on_message from {:?}: {} bytes",
            peer_id,
            data.len()
        );

        self.events
            .lock()
            .await
            .push(RoomEvent::Message { peer_id, data });
    }

    async fn on_leave(&self, _ctx: &RoomContext, peer_id: PeerId) {
        tracing::info!("[TestBehavior] on_leave: {:?}", peer_id);

        self.events.lock().await.push(RoomEvent::Leave { peer_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_behavior_records_events() {
        let behavior = TestRoomBehavior::new();
        let peer_id = PeerId::new();

        // Simulate events (we can't call on_join directly without RoomContext,
        // but we can test the event recording mechanism)
        behavior.events.lock().await.push(RoomEvent::Join {
            peer_id: peer_id.clone(),
        });

        assert!(behavior.has_join(&peer_id).await);
        assert!(!behavior.has_leave(&peer_id).await);
    }
}
