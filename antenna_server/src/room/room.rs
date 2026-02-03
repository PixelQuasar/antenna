use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// WebRTC imports
use webrtc::data_channel::RTCDataChannel;

// Internal imports
use crate::room::context::RoomContext;
use crate::room::room_behavior::RoomBehavior;
use crate::room::room_command::RoomCommand;
use crate::signaling::SignalingOutput;
use crate::transport::{ConnectionWrapper, TransportConfig, TransportEvent};
use antenna_core::model::PeerId;

/// Основной актор комнаты.
/// Управляет состоянием, пирами и вызывает пользовательскую логику.
pub struct Room {
    /// Пользовательская логика (геймплей).
    behavior: Box<dyn RoomBehavior>,

    /// Активные каналы для передачи данных (Context разделяет доступ к ним).
    /// Используем DashMap для потокобезопасного доступа из Context.
    peers_data: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>,

    /// Активные транспорты (PeerConnection).
    /// Храним их здесь, чтобы соединение оставалось живым.
    transports: HashMap<PeerId, ConnectionWrapper>,

    /// Канал для приема команд (извне).
    command_rx: mpsc::Receiver<RoomCommand>,

    /// Канал для приема событий от транспортов (внутренний).
    transport_rx: mpsc::Receiver<TransportEvent>,

    /// Отправитель для internal событий (клонируем и передаем в WrtcTransport).
    transport_tx: mpsc::Sender<TransportEvent>,

    /// Интерфейс для отправки signaling-сообщений наружу.
    signaling: Box<dyn SignalingOutput>,

    /// Настройки WebRTC (STUN/TURN).
    transport_config: TransportConfig,
}

impl Room {
    pub fn new(
        behavior: Box<dyn RoomBehavior>,
        command_rx: mpsc::Receiver<RoomCommand>,
        signaling: Box<dyn SignalingOutput>,
    ) -> Self {
        // Создаем канал MPSC для сбора событий от всех WebRTC подключений в один поток
        let (transport_tx, transport_rx) = mpsc::channel(256);

        Self {
            behavior,
            peers_data: Arc::new(DashMap::new()),
            transports: HashMap::new(),
            command_rx,
            transport_rx,
            transport_tx,
            signaling,
            transport_config: TransportConfig::default(),
        }
    }

    /// Запуск главного цикла (Event Loop) комнаты.
    /// Блокирующий метод (асинхронно), должен быть запущен через tokio::spawn.
    pub async fn run(mut self) {
        info!("Room event loop started");

        loop {
            // Создаем временный контекст для передачи в RoomBehavior.
            // Это дешевая операция (клонирование Arc).
            let ctx = RoomContext::new(self.peers_data.clone());

            tokio::select! {
                // 1. Обработка внешних команд (Signaling -> Room)
            cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(c) => self.handle_command(c).await,
                        None => {
                            info!("Command channel closed. Shutting down room.");
                            break;
                        }
                    }
                }

                // 2. Обработка событий от WebRTC (Peer -> Room)
                evt = self.transport_rx.recv() => {
                    match evt {
                        Some(e) => self.handle_transport_event(e, &ctx).await,
                        None => {
                            // Теоретически недостижимо, так как tx хранится внутри структуры
                            warn!("Transport channel closed unexpectedly");
                            break;
                        }
                    }
                }
            }
        }

        info!("Room event loop finished");
    }

    /// Обработка команд от сигнального слоя.
    async fn handle_command(&mut self, cmd: RoomCommand) {
        match cmd {
            RoomCommand::JoinRequest { peer_id, offer } => {
                info!("Processing JoinRequest for user {:?}", peer_id);

                // Если пользователь уже есть (например, реконнект), очищаем старое
                if self.transports.contains_key(&peer_id) {
                    self.remove_peer(&peer_id).await;
                }

                // 1. Создаем новый транспорт
                let transport_res = ConnectionWrapper::new(
                    peer_id.clone(),
                    self.transport_config.clone(),
                    self.transport_tx.clone(),
                )
                .await;

                match transport_res {
                    Ok(transport) => {
                        // 2. Устанавливаем удаленный дескрипшн (Offer)
                        if let Err(e) = transport.set_remote_description(offer).await {
                            error!("SDP error for {:?}: {:?}", peer_id, e);
                            return;
                        }

                        // 3. Создаем локальный дескрипшн (Answer)
                        match transport.create_answer().await {
                            Ok(answer_sdp) => {
                                // 4. Сохраняем транспорт, чтобы соединение жило
                                self.transports.insert(peer_id.clone(), transport);

                                // 5. Отправляем ответ клиенту через WebSocket
                                self.signaling.send_answer(peer_id, answer_sdp).await;
                            }
                            Err(e) => error!("Failed to create answer for {:?}: {:?}", peer_id, e),
                        }
                    }
                    Err(e) => error!("Failed to create transport for {:?}: {:?}", peer_id, e),
                }
            }

            RoomCommand::IceCandidate { peer_id, candidate } => {
                let Some(transport) = self.transports.get(&peer_id) else {
                    return;
                };
                let Err(e) = transport.add_ice_candidate(candidate).await else {
                    return;
                };
                warn!("Failed to add ICE candidate for {:?}: {:?}", peer_id, e);
            }

            RoomCommand::Disconnect { peer_id } => {
                // Удаление инициировано сигнальным слоем (закрыл вкладку)
                self.remove_peer_with_notify(&peer_id, &RoomContext::new(self.peers_data.clone()))
                    .await;
            }
        }
    }

    /// Обработка событий транспортного слоя WebRTC.
    async fn handle_transport_event(&mut self, event: TransportEvent, ctx: &RoomContext) {
        match event {
            TransportEvent::DataChannelReady(peer_id, channel) => {
                info!("User {:?} fully joined (DataChannel ready).", peer_id);

                // 1. Сохраняем канал в общуюкарту, чтобы Context мог слать данные
                self.peers_data.insert(peer_id.clone(), channel);

                // 2. Уведомляем игровую логику о входе игрока
                self.behavior.on_join(ctx, peer_id).await;
            }

            TransportEvent::Message(peer_id, data) => {
                // Прокидываем сырые байты в логику
                self.behavior.on_message(ctx, peer_id, data).await;
            }

            TransportEvent::Disconnected(peer_id) => {
                // Разрыв WebRTC соединения (таймаут или ошибка сети)
                info!("Transport disconnected for {:?}", peer_id);
                self.remove_peer_with_notify(&peer_id, ctx).await;
            }

            TransportEvent::CandidateGenerated(peer_id, candidate_json) => {
                // WebRTC нашел локальный путь, отправляем его клиенту через Signaling
                self.signaling.send_ice(peer_id, candidate_json).await;
            }
        }
    }

    /// Полное удаление пира с вызовом on_leave
    async fn remove_peer_with_notify(&mut self, peer_id: &PeerId, ctx: &RoomContext) {
        // Проверяем, был ли пир "активен" (с открытым каналом), чтобы не спамить on_leave лишний раз
        let was_active = self.peers_data.contains_key(peer_id);

        self.remove_peer(peer_id).await;

        if was_active {
            self.behavior.on_leave(ctx, peer_id.clone()).await;
        }
    }

    /// Техническое удаление пира из структур
    async fn remove_peer(&mut self, peer_id: &PeerId) {
        // Удаляем возможность отправлять ему данные
        self.peers_data.remove(peer_id);

        // Закрываем PeerConnection и удаляем транспорт
        let Some(transport) = self.transports.remove(peer_id) else {
            return;
        };
        let _ = transport.close().await;
    }
}
