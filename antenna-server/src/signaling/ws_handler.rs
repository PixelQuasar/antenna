use crate::{RoomCommand, SignalingService};
use antenna_core::{PeerId, SignalMessage};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

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
