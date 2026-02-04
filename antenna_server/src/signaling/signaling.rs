use crate::room::RoomCommand;
use crate::signaling::SignalingOutput;
use antenna_core::{PeerId, SignalMessage};
use async_trait::async_trait;
use axum::{
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

struct SignalingInner {
    peers: DashMap<PeerId, mpsc::UnboundedSender<Message>>,
}

#[derive(Clone)]
pub struct SignalingService {
    inner: Arc<SignalingInner>,
    room_cmd_tx: mpsc::Sender<RoomCommand>,
}

impl SignalingService {
    pub fn new(room_cmd_tx: mpsc::Sender<RoomCommand>) -> Self {
        Self {
            inner: Arc::new(SignalingInner {
                peers: DashMap::new(),
            }),
            room_cmd_tx,
        }
    }

    fn add_peer(&self, peer_id: PeerId, tx: mpsc::UnboundedSender<Message>) {
        self.inner.peers.insert(peer_id, tx);
    }

    fn remove_peer(&self, peer_id: &PeerId) {
        self.inner.peers.remove(peer_id);
    }

    fn send_signal(&self, peer_id: PeerId, msg: SignalMessage) {
        if let Some(peer) = self.inner.peers.get(&peer_id) {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(e) = peer.send(Message::Text(json.into())) {
                        error!("Failed to send WS message to {:?}: {:?}", peer_id, e);
                    }
                }
                Err(e) => error!("Failed to serialize signal message: {}", e),
            }
        } else {
            warn!(
                "Attempted to send signal to disconnected user {:?}",
                peer_id
            );
        }
    }
}

#[async_trait]
impl SignalingOutput for SignalingService {
    async fn send_answer(&self, peer_id: PeerId, sdp: String) {
        let msg = SignalMessage::Answer { sdp };
        self.send_signal(peer_id, msg);
    }

    async fn send_ice(&self, peer_id: PeerId, candidate: String) {
        let msg = SignalMessage::IceCandidate {
            candidate,
            sdp_mid: None,
            sdp_m_line_index: None,
        };
        self.send_signal(peer_id, msg);
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(peer_id): Path<String>,
    State(service): State<SignalingService>,
) -> impl IntoResponse {
    let peer_id = PeerId::from(peer_id);

    ws.on_upgrade(move |socket| handle_socket(socket, peer_id, service))
}

async fn handle_socket(socket: WebSocket, peer_id: PeerId, service: SignalingService) {
    info!("New WebSocket connection: {:?}", peer_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    service.add_peer(peer_id.clone(), tx);

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn({
        let service = service.clone();
        let peer_id = peer_id.clone();

        async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => match serde_json::from_str::<SignalMessage>(&text) {
                        Ok(signal) => match signal {
                            SignalMessage::Offer { sdp } => {
                                let cmd = RoomCommand::JoinRequest {
                                    peer_id: peer_id.clone(),
                                    offer: sdp,
                                };
                                if let Err(e) = service.room_cmd_tx.send(cmd).await {
                                    error!("Room died: {}", e);
                                    break;
                                }
                            }
                            SignalMessage::IceCandidate { candidate, .. } => {
                                let cmd = RoomCommand::IceCandidate {
                                    peer_id: peer_id.clone(),
                                    candidate,
                                };
                                let _ = service.room_cmd_tx.send(cmd).await;
                            }
                            SignalMessage::Join { room, .. } => {
                                info!("Peer {:?} wants to join room '{}'", peer_id, room);
                            }
                            _ => {}
                        },
                        Err(e) => warn!("Invalid SignalMessage from {:?}: {:?}", peer_id, e),
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            let _ = service
                .room_cmd_tx
                .send(RoomCommand::Disconnect {
                    peer_id: peer_id.clone(),
                })
                .await;
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    service.remove_peer(&peer_id);
    info!("WebSocket disconnected: {:?}", peer_id);
}
