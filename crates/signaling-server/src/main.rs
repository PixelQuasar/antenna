use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

use antenna_client_shared::{ClientMsg, ServerMsg};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

enum RoomState {
    WaitingForOffer {
        host_tx: mpsc::UnboundedSender<String>,
    },
    WaitingForJoiner {
        offer: String,
        host_tx: mpsc::UnboundedSender<String>,
    },
    WaitingForAnswer {
        host_tx: mpsc::UnboundedSender<String>,
        _joiner_tx: mpsc::UnboundedSender<String>,
    },
}

fn serialize(msg: ServerMsg<'_>) -> String {
    serde_json::to_string(&msg).expect("ServerMsg is always serializable")
}

#[tokio::main]
async fn main() {
    let addr = env::var("ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let rooms = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/ws", axum::routing::get(ws_handler))
        .with_state(rooms);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Signaling server listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(rooms): State<Arc<Mutex<HashMap<String, RoomState>>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, rooms))
}

async fn handle_socket(socket: WebSocket, rooms: Arc<Mutex<HashMap<String, RoomState>>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (mut ws_tx, mut ws_rx) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let mut joined_room: Option<String> = None;

    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: ClientMsg<'_> = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let err = e.to_string();
                let _ = tx.send(serialize(ServerMsg::Error { message: &err }));
                continue;
            }
        };

        match client_msg {
            ClientMsg::Join { room_id } => {
                let mut rooms = rooms.lock().unwrap();
                if !rooms.contains_key(room_id) {
                    rooms.insert(
                        room_id.to_owned(),
                        RoomState::WaitingForOffer {
                            host_tx: tx.clone(),
                        },
                    );
                    joined_room = Some(room_id.to_owned());
                    let _ = tx.send(serialize(ServerMsg::RequestOffer));
                } else {
                    match rooms.remove(room_id).unwrap() {
                        RoomState::WaitingForJoiner { offer, host_tx } => {
                            rooms.insert(
                                room_id.to_owned(),
                                RoomState::WaitingForAnswer {
                                    host_tx,
                                    _joiner_tx: tx.clone(),
                                },
                            );
                            joined_room = Some(room_id.to_owned());
                            let _ = tx.send(serialize(ServerMsg::OfferReceived { offer: &offer }));
                        }
                        other => {
                            rooms.insert(room_id.to_owned(), other);
                            let _ = tx.send(serialize(ServerMsg::Error {
                                message: "Room is not ready for a new peer",
                            }));
                        }
                    }
                }
            }
            ClientMsg::Offer { room_id, offer } => {
                let mut rooms = rooms.lock().unwrap();
                match rooms.remove(room_id) {
                    Some(RoomState::WaitingForOffer { host_tx }) => {
                        rooms.insert(
                            room_id.to_owned(),
                            RoomState::WaitingForJoiner {
                                offer: offer.to_owned(),
                                host_tx,
                            },
                        );
                    }
                    other => {
                        if let Some(state) = other {
                            rooms.insert(room_id.to_owned(), state);
                        }
                        let _ = tx.send(serialize(ServerMsg::Error {
                            message: "Unexpected offer",
                        }));
                    }
                }
            }
            ClientMsg::Answer { room_id, answer } => {
                let mut rooms = rooms.lock().unwrap();
                match rooms.remove(room_id) {
                    Some(RoomState::WaitingForAnswer { host_tx, .. }) => {
                        let _ = host_tx.send(serialize(ServerMsg::AnswerReceived { answer }));
                    }
                    other => {
                        if let Some(state) = other {
                            rooms.insert(room_id.to_owned(), state);
                        }
                        let _ = tx.send(serialize(ServerMsg::Error {
                            message: "Unexpected answer",
                        }));
                    }
                }
            }
        }
    }

    if let Some(room_id) = joined_room {
        rooms.lock().unwrap().remove(&room_id);
    }

    send_task.abort();
}
