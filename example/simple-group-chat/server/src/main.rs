use async_trait::async_trait;
use axum::{Router, routing::get};
use bytes::Bytes;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{Level, info};

use antenna::PeerId;
use antenna::server::{
    Room,             // Оркестратор комнаты
    RoomBehavior,     // Трейт для нашей логики
    RoomCommand,      // Команды (для связи main -> room)
    RoomContext,      // Инструмент отправки сообщений
    SignalingService, // WebSocket упралвение
    ws_handler,       // Готовый хендлер для Axum
};

struct ChatRoom;

#[async_trait]
impl RoomBehavior for ChatRoom {
    /// Вызывается, когда P2P соединение полностью установлено (DataChannel открыт)
    async fn on_join(&self, ctx: &RoomContext, user_id: PeerId) {
        info!(">>> User joined the chat: {:?}", user_id);

        // 1. Отправляем приветствие только этому пользователю (Unicast)
        let welcome = format!("Welcome to Antenna Chat, {:?}!", user_id);
        ctx.send(&user_id, Bytes::from(welcome)).await;

        // 2. Уведомляем остальных, что пришел новичок (Broadcast)
        let announcement = format!("System: User {:?} has joined.", user_id);
        ctx.broadcast(Bytes::from(announcement)).await;
    }

    /// Вызывается, когда приходят бинарные данные через WebRTC
    async fn on_message(&self, ctx: &RoomContext, user_id: PeerId, data: Bytes) {
        // Предполагаем, что клиент шлет UTF-8 текст
        let text = String::from_utf8_lossy(&data);
        info!("Msg from {:?}: {:?}", user_id, text);

        // Форматируем сообщение: "[ID]: Text"
        let broadcast_msg = format!("[{}:?]: {:?}", user_id, text);

        // Рассылаем всем подключенным (включая отправителя)
        ctx.broadcast(Bytes::from(broadcast_msg)).await;
    }

    /// Вызывается при разрыве соединения
    async fn on_leave(&self, _ctx: &RoomContext, user_id: PeerId) {
        info!("<<< User left the chat: {}", user_id);
    }
}

#[tokio::main]
async fn main() {
    // 1. Настройка логирования (в консоль)
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Initializing Chat Server...");

    // 2. Канал связи между веб-сервером (Signaling) и игровой комнатой (Room)
    // Signaling пишет команды сюда, Room их читает.
    let (cmd_tx, cmd_rx) = mpsc::channel::<RoomCommand>(100);

    // 3. Создаем Signaling Service
    // Он держит соединения WebSocket и пересылает команды в cmd_tx
    let signaling = SignalingService::new(cmd_tx);

    // 4. Создаем Комнату
    // Передаем ей нашу логику (ChatRoom), входящий канал команд и "выхлоп" сигналинга (для отправки ответов)
    let room = NexusRoom::new(
        Box::new(ChatRoom),
        cmd_rx,
        Box::new(signaling.clone()), // Клонируем сервис (внутри Arc), чтобы передать владение
    );

    // 5. Запускаем "Вечный цикл" комнаты в отдельном потоке
    tokio::spawn(async move {
        room.run().await;
    });
    info!("Room loop started.");

    // 6. Настройка HTTP сервера (Axum)

    // Важно: разрешаем CORS, иначе браузерный клиент (с другого порта) не сможет подключиться
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Маршрут websocket: ws://localhost:3000/ws/{uuid}
        // ws_handler импортирован из antenna::server
        .route("/ws/:user_id", get(ws_handler))
        .layer(cors)
        .with_state(signaling); // Передаем состояние (SignalingService) в хендлеры

    // 7. Запуск слушателя
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Signaling server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
