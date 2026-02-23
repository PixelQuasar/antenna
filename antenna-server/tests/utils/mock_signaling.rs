use antenna_core::{PeerId, SignalMessage};
use antenna_server::SignalingService;
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Mock SignalingOutput that captures all outgoing signals.
#[derive(Clone)]
pub struct MockSignalingOutput {
    /// Channel to send captured signals.
    tx: mpsc::UnboundedSender<SignalMessage>,
    /// All captured signals (for verification).
    signals: Arc<Mutex<Vec<SignalMessage>>>,
    /// The actual SignalingService instance
    pub service: Arc<SignalingService>,
}

impl MockSignalingOutput {
    /// Create a new MockSignalingOutput and its receiver channel.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SignalMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let service = Arc::new(SignalingService::new(vec![]));

        let signaling = Self {
            tx,
            signals: Arc::new(Mutex::new(Vec::new())),
            service,
        };
        (signaling, rx)
    }

    /// Create a MockSignalingOutput without a receiver (signals are only stored).
    pub fn new_stored_only() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        let service = Arc::new(SignalingService::new(vec![]));

        Self {
            tx,
            signals: Arc::new(Mutex::new(Vec::new())),
            service,
        }
    }

    /// Register a peer to capture its messages
    pub fn register_peer(&self, peer_id: PeerId) {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        self.service.add_peer(peer_id.clone(), ws_tx);

        let tx = self.tx.clone();
        let signals = self.signals.clone();

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let Message::Text(text) = msg {
                    if let Ok(signal) = serde_json::from_str::<SignalMessage>(&text) {
                        signals.lock().await.push(signal.clone());
                        let _ = tx.send(signal);
                    }
                }
            }
        });
    }

    /// Get the SDP answer for a specific peer (if any).
    pub async fn get_answer_for(&self, peer_id: &PeerId) -> Option<String> {
        self.signals.lock().await.iter().find_map(|s| match s {
            SignalMessage::Answer { sdp } => Some(sdp.clone()),
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
                SignalMessage::IceCandidate { candidate } => Some(candidate.clone()),
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
