use crate::{RoomCommand, RoomManager, SignalingService};
use antenna_core::{PeerId, SignalMessage};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pub signaling: SignalingService,
    pub room_manager: RoomManager,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(peer_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let peer_id = PeerId::from(peer_id);

    ws.on_upgrade(move |socket| handle_socket(socket, peer_id, state))
}

async fn handle_socket(socket: WebSocket, peer_id: PeerId, state: Arc<AppState>) {
    let service = &state.signaling;
    info!("New WebSocket connection: {:?}", peer_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    service.add_peer(peer_id.clone(), tx);

    // Send ICE configuration immediately
    let ice_servers = service.get_ice_servers();
    if !ice_servers.is_empty() {
        service.send_signal(peer_id.clone(), SignalMessage::IceConfig { ice_servers });
    }

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // println!("MSG {:?}", msg);
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn({
        let service = service.clone();
        let peer_id = peer_id.clone();
        let state = state.clone();

        async move {
            let mut current_room_tx: Option<mpsc::Sender<RoomCommand>> = None;

            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => match serde_json::from_str::<SignalMessage>(&text) {
                        Ok(signal) => match signal {
                            SignalMessage::Join { room, .. } => {
                                info!("Peer {:?} wants to join room '{}'", peer_id, room);
                                current_room_tx = Some(state.room_manager.get_room_sender(&room));

                                service.send_signal(
                                    peer_id.clone(),
                                    SignalMessage::Welcome {
                                        peer_id: peer_id.clone(),
                                    },
                                )
                            }
                            SignalMessage::Offer { sdp } => {
                                if let Some(tx) = &current_room_tx {
                                    let cmd = RoomCommand::JoinRequest {
                                        peer_id: peer_id.clone(),
                                        offer: sdp,
                                    };
                                    info!("{:?}", cmd);
                                    if let Err(e) = tx.send(cmd).await {
                                        error!("Room died: {}", e);
                                        break;
                                    }
                                } else {
                                    warn!("Peer {:?} sent Offer without joining a room", peer_id);
                                }
                            }
                            SignalMessage::IceCandidate { candidate, .. } => {
                                if let Some(tx) = &current_room_tx {
                                    let cmd = RoomCommand::IceCandidate {
                                        peer_id: peer_id.clone(),
                                        candidate,
                                    };
                                    info!("{:?}", cmd);
                                    let _ = tx.send(cmd).await;
                                }
                            }
                            _ => {}
                        },
                        Err(e) => warn!("Invalid SignalMessage from {:?}: {:?}", peer_id, e),
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            if let Some(tx) = current_room_tx {
                let _ = tx
                    .send(RoomCommand::Disconnect {
                        peer_id: peer_id.clone(),
                    })
                    .await;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    service.remove_peer(&peer_id);
    info!("WebSocket disconnected: {:?}", peer_id);
}
