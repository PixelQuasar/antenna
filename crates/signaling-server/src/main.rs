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
use std::{
    collections::HashMap,
    env,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::mpsc;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);
type Sender = mpsc::UnboundedSender<String>;

#[derive(Clone)]
struct Conn {
    id: u64,
    tx: Sender,
}

enum RoomState {
    WaitingForOffer {
        host: Conn,
        joiner: Option<Conn>,
        brokers: Vec<Conn>,
    },
    WaitingForJoiner {
        offer: String,
        host: Conn,
        brokers: Vec<Conn>,
    },
    WaitingForAnswer {
        host: Conn,
        joiner: Conn,
        brokers: Vec<Conn>,
    },
    Ready {
        brokers: Vec<Conn>,
    },
}

impl RoomState {
    fn remove_broker(&mut self, id: u64) {
        let brokers = match self {
            RoomState::WaitingForOffer { brokers, .. } => brokers,
            RoomState::WaitingForJoiner { brokers, .. } => brokers,
            RoomState::WaitingForAnswer { brokers, .. } => brokers,
            RoomState::Ready { brokers } => brokers,
        };
        brokers.retain(|c| c.id != id);
    }
}

fn serialize(msg: ServerMsg<'_>) -> String {
    serde_json::to_string(&msg).expect("ServerMsg is always serializable")
}

#[tokio::main]
async fn main() {
    let addr = env::var("ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let rooms: Arc<Mutex<HashMap<String, RoomState>>> = Arc::new(Mutex::new(HashMap::new()));

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
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
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

        let me = Conn {
            id: conn_id,
            tx: tx.clone(),
        };

        match client_msg {
            ClientMsg::Join { room_id } => {
                let mut rooms = rooms.lock().unwrap();

                match rooms.remove(room_id) {
                    None => {
                        rooms.insert(
                            room_id.to_owned(),
                            RoomState::WaitingForOffer {
                                host: me.clone(),
                                joiner: None,
                                brokers: vec![],
                            },
                        );
                        joined_room = Some(room_id.to_owned());
                        let _ = me.tx.send(serialize(ServerMsg::RequestOffer));
                    }
                    Some(RoomState::WaitingForJoiner {
                        offer,
                        host,
                        brokers,
                    }) => {
                        rooms.insert(
                            room_id.to_owned(),
                            RoomState::WaitingForAnswer {
                                host,
                                joiner: me.clone(),
                                brokers,
                            },
                        );
                        joined_room = Some(room_id.to_owned());
                        let _ = me
                            .tx
                            .send(serialize(ServerMsg::OfferReceived { offer: &offer }));
                    }
                    Some(RoomState::Ready { mut brokers }) => {
                        brokers.retain(|b| !b.tx.is_closed());
                        if let Some(broker) = brokers.first().cloned() {
                            let remaining = brokers[1..].to_vec();
                            rooms.insert(
                                room_id.to_owned(),
                                RoomState::WaitingForOffer {
                                    host: broker.clone(),
                                    joiner: Some(me.clone()),
                                    brokers: remaining,
                                },
                            );
                            joined_room = Some(room_id.to_owned());
                            let _ = broker.tx.send(serialize(ServerMsg::RequestOffer));
                        } else {
                            // All brokers gone, restart room
                            rooms.insert(
                                room_id.to_owned(),
                                RoomState::WaitingForOffer {
                                    host: me.clone(),
                                    joiner: None,
                                    brokers: vec![],
                                },
                            );
                            joined_room = Some(room_id.to_owned());
                            let _ = me.tx.send(serialize(ServerMsg::RequestOffer));
                        }
                    }
                    Some(other) => {
                        rooms.insert(room_id.to_owned(), other);
                        let _ = me.tx.send(serialize(ServerMsg::Error {
                            message: "Room is busy, try again shortly",
                        }));
                    }
                }
            }

            ClientMsg::Offer { room_id, offer } => {
                let mut rooms = rooms.lock().unwrap();
                match rooms.remove(room_id) {
                    Some(RoomState::WaitingForOffer {
                        host,
                        joiner: Some(joiner),
                        brokers,
                    }) => {
                        rooms.insert(
                            room_id.to_owned(),
                            RoomState::WaitingForAnswer {
                                host,
                                joiner: joiner.clone(),
                                brokers,
                            },
                        );
                        let _ = joiner
                            .tx
                            .send(serialize(ServerMsg::OfferReceived { offer }));
                    }
                    Some(RoomState::WaitingForOffer {
                        host,
                        joiner: None,
                        brokers,
                    }) => {
                        rooms.insert(
                            room_id.to_owned(),
                            RoomState::WaitingForJoiner {
                                offer: offer.to_owned(),
                                host,
                                brokers,
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
                    Some(RoomState::WaitingForAnswer {
                        host,
                        joiner,
                        mut brokers,
                    }) => {
                        let _ = host
                            .tx
                            .send(serialize(ServerMsg::AnswerReceived { answer }));
                        brokers.push(host);
                        brokers.push(joiner);
                        rooms.insert(room_id.to_owned(), RoomState::Ready { brokers });
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
        let mut rooms = rooms.lock().unwrap();
        if let Some(state) = rooms.get_mut(&room_id) {
            state.remove_broker(conn_id);
            if let RoomState::Ready { brokers } = state {
                if brokers.is_empty() {
                    rooms.remove(&room_id);
                }
            }
        }
    }

    send_task.abort();
}
