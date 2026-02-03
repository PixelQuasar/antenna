use anyhow::Result;
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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::room::RoomCommand;
use crate::signaling::SignalingOutput;
use antenna_core::model::PeerId;
// --- Протокол обмена (JSON) ---

/// Сообщения, приходящие от клиента (браузера)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum IncomingSignal {
    /// Клиент хочет присоединиться (отправляет SDP Offer)
    Join { offer: String },
    /// Клиент отправляет найденного ICE кандидата
    Ice { candidate: String },
}

/// Сообщения, отправляемые клиенту (браузеру)
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum OutgoingSignal {
    /// Ответ сервера (SDP Answer)
    Answer { sdp: String },
    /// ICE кандидат от сервера
    Ice { candidate: String },
}

// --- Состояние сервиса ---

/// Внутреннее состояние Signaling сервиса.
/// Хранит активные WebSocket-соединения для отправки ответов.
struct SignalingInner {
    /// Карта активных соединений: UserId -> Канал отправки в WebSocket
    peers: DashMap<PeerId, mpsc::UnboundedSender<Message>>,
}

/// Обертка над состоянием, которую будем клонировать и передавать.
#[derive(Clone)]
pub struct SignalingService {
    inner: Arc<SignalingInner>,
    /// Канал для отправки команд в NexusRoom (RoomCommand)
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

    /// Добавить нового пира в список рассылки
    fn add_peer(&self, peer_id: PeerId, tx: mpsc::UnboundedSender<Message>) {
        self.inner.peers.insert(peer_id, tx);
    }

    /// Удалить пира
    fn remove_peer(&self, peer_id: &PeerId) {
        self.inner.peers.remove(peer_id);
    }
}

#[async_trait]
impl SignalingOutput for SignalingService {
    async fn send_answer(&self, peer_id: PeerId, sdp: String) {
        let msg = OutgoingSignal::Answer { sdp };
        self.send_json(peer_id, msg);
    }

    async fn send_ice(&self, peer_id: PeerId, candidate: String) {
        let msg = OutgoingSignal::Ice { candidate };
        self.send_json(peer_id, msg);
    }
}

impl SignalingService {
    /// Вспомогательный метод для сериализации и отправки JSON в WebSocket
    fn send_json(&self, peer_id: PeerId, msg: OutgoingSignal) {
        if let Some(peer) = self.inner.peers.get(&peer_id) {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    // Отправляем текстовое сообщение
                    // unbounded_send не блокирует поток, что хорошо для DashMap read lock
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

// --- Axum Handler ---

/// HTTP-хендлер для апгрейда до WebSocket.
/// Пример маршрута: GET /ws/:user_id
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(peer_id): Path<String>,             // Достаем user_id из URL
    State(service): State<SignalingService>, // State из Axum Router
) -> impl IntoResponse {
    let peer_id = PeerId::from(peer_id); // Преобразуем String в UserId (NexusCore)

    ws.on_upgrade(move |socket| handle_socket(socket, peer_id, service))
}

/// Логика обработки конкретного WebSocket соединения
async fn handle_socket(socket: WebSocket, peer_id: PeerId, service: SignalingService) {
    info!("New WebSocket connection: {:?}", peer_id);

    // Разделяем сокет на чтение (receiver) и запись (sender)
    let (mut sender, mut receiver) = socket.split();

    // Создаем внутренний канал mpsc, чтобы service.send_json мог писать в этот сокет.
    // mpsc -> sender (websocket sink)
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Регистрируем пира в SignalingService, чтобы Room могла ему отвечать
    service.add_peer(peer_id.clone(), tx);

    // Запускаем задачу по пересылке сообщений из mpsc в WebSocket sender
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break; // Клиент отключился
            }
        }
    });

    // Главный цикл чтения сообщений ОТ клиента
    let mut recv_task = tokio::spawn({
        let service = service.clone();
        let peer_id = peer_id.clone();

        async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        // 1. Десериализуем JSON
                        match serde_json::from_str::<IncomingSignal>(&text) {
                            Ok(signal) => {
                                // 2. Преобразуем в команду для Room
                                let cmd = match signal {
                                    IncomingSignal::Join { offer } => RoomCommand::JoinRequest {
                                        peer_id: peer_id.clone(),
                                        offer,
                                    },
                                    IncomingSignal::Ice { candidate } => {
                                        RoomCommand::IceCandidate {
                                            peer_id: peer_id.clone(),
                                            candidate,
                                        }
                                    }
                                };

                                // 3. Отправляем в NexusRoom
                                if let Err(e) = service.room_cmd_tx.send(cmd).await {
                                    error!("Failed to send command to room: {}", e);
                                    break; // Комната умерла
                                }
                            }
                            Err(e) => warn!("Invalid JSON from {:?}: {:?}", peer_id, e),
                        }
                    }
                    Message::Close(_) => break,
                    _ => {} // Игнорируем Ping/Pong/Binary в сигнальном слое
                }
            }

            // Завершение соединения -> отправляем Disconnect в комнату
            let _ = service
                .room_cmd_tx
                .send(RoomCommand::Disconnect {
                    peer_id: peer_id.clone(),
                })
                .await;
        }
    });

    // Ожидаем завершения любой из задач (чтение или запись)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Очистка
    service.remove_peer(&peer_id);
    info!("WebSocket disconnected: {:?}", peer_id);
}
