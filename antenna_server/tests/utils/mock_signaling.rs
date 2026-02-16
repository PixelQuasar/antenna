use antenna_core::PeerId;
use antenna_server::SignalingOutput;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// A signal message sent from Room to a peer.
#[derive(Debug, Clone)]
pub enum SignalMessage {
    /// SDP Answer to be sent to the peer.
    Answer { peer_id: PeerId, sdp: String },
    /// ICE Candidate to be sent to the peer.
    Ice { peer_id: PeerId, candidate: String },
}

/// Mock SignalingOutput that captures all outgoing signals.
///
/// # Example
///
/// ```ignore
/// let (signaling, mut rx) = MockSignalingOutput::new();
///
/// // ... room sends answer ...
///
/// if let Some(SignalMessage::Answer { peer_id, sdp }) = rx.recv().await {
///     // Forward to test client
/// }
/// ```
#[derive(Clone)]
pub struct MockSignalingOutput {
    /// Channel to send captured signals.
    tx: mpsc::UnboundedSender<SignalMessage>,
    /// All captured signals (for verification).
    signals: Arc<Mutex<Vec<SignalMessage>>>,
}

impl MockSignalingOutput {
    /// Create a new MockSignalingOutput and its receiver channel.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SignalMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let signaling = Self {
            tx,
            signals: Arc::new(Mutex::new(Vec::new())),
        };
        (signaling, rx)
    }

    /// Create a MockSignalingOutput without a receiver (signals are only stored).
    pub fn new_stored_only() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self {
            tx,
            signals: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all captured signals.
    pub async fn get_signals(&self) -> Vec<SignalMessage> {
        self.signals.lock().await.clone()
    }

    /// Get the SDP answer for a specific peer (if any).
    pub async fn get_answer_for(&self, peer_id: &PeerId) -> Option<String> {
        self.signals.lock().await.iter().find_map(|s| match s {
            SignalMessage::Answer { peer_id: id, sdp } if id == peer_id => Some(sdp.clone()),
            _ => None,
        })
    }

    /// Get all ICE candidates for a specific peer.
    pub async fn get_ice_candidates_for(&self, peer_id: &PeerId) -> Vec<String> {
        self.signals
            .lock()
            .await
            .iter()
            .filter_map(|s| match s {
                SignalMessage::Ice {
                    peer_id: id,
                    candidate,
                } if id == peer_id => Some(candidate.clone()),
                _ => None,
            })
            .collect()
    }

    /// Wait for an answer for a specific peer with timeout.
    pub async fn wait_for_answer(&self, peer_id: &PeerId, timeout_ms: u64) -> Option<String> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            if let Some(answer) = self.get_answer_for(peer_id).await {
                return Some(answer);
            }
            if start.elapsed() > timeout {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

impl Default for MockSignalingOutput {
    fn default() -> Self {
        Self::new_stored_only()
    }
}

#[async_trait]
impl SignalingOutput for MockSignalingOutput {
    async fn send_answer(&self, peer_id: PeerId, sdp: String) {
        tracing::debug!("[MockSignaling] send_answer to {:?}", peer_id);

        let msg = SignalMessage::Answer {
            peer_id: peer_id.clone(),
            sdp: sdp.clone(),
        };

        self.signals.lock().await.push(msg.clone());
        let _ = self.tx.send(msg);
    }

    async fn send_ice(&self, peer_id: PeerId, candidate: String) {
        tracing::debug!("[MockSignaling] send_ice to {:?}", peer_id);

        let msg = SignalMessage::Ice {
            peer_id: peer_id.clone(),
            candidate: candidate.clone(),
        };

        self.signals.lock().await.push(msg.clone());
        let _ = self.tx.send(msg);
    }
}

/// Helper to create a channel-based signaling pair.
///
/// Returns (MockSignalingOutput, receiver for signals).
pub fn create_signaling_pair() -> (MockSignalingOutput, mpsc::UnboundedReceiver<SignalMessage>) {
    MockSignalingOutput::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_signaling_captures_answer() {
        let (signaling, mut rx) = MockSignalingOutput::new();
        let peer_id = PeerId::new();
        let sdp = "test-sdp".to_string();

        signaling.send_answer(peer_id.clone(), sdp.clone()).await;

        // Check via channel
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, SignalMessage::Answer { .. }));

        // Check via stored signals
        let answer = signaling.get_answer_for(&peer_id).await;
        assert_eq!(answer, Some(sdp));
    }

    #[tokio::test]
    async fn test_mock_signaling_captures_ice() {
        let signaling = MockSignalingOutput::new_stored_only();
        let peer_id = PeerId::new();
        let candidate = "candidate:123".to_string();

        signaling.send_ice(peer_id.clone(), candidate.clone()).await;

        let candidates = signaling.get_ice_candidates_for(&peer_id).await;
        assert_eq!(candidates, vec![candidate]);
    }
}
