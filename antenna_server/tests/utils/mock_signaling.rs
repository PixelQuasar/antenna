use antenna_core::PeerId;
use antenna_server::SignalingOutput;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Debug, Clone)]
pub enum SignalMessage {
    Answer { peer_id: PeerId, sdp: String },
    Ice { peer_id: PeerId, candidate: String },
}

/// Mock SignalingOutput that captures all outgoing signals.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_signaling_captures_answer() {
        let (signaling, mut rx) = MockSignalingOutput::new();
        let peer_id = PeerId::new();
        let sdp = "test-sdp".to_string();

        signaling.send_answer(peer_id.clone(), sdp.clone()).await;

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, SignalMessage::Answer { .. }));

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
