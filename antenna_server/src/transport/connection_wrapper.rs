use crate::transport::transport_config::TransportConfig;
use crate::transport::transport_event::TransportEvent;
use antenna_core::model::PeerId;
use anyhow::{Context, Result};
use bytes::Bytes;
use std::default::Default;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

pub struct ConnectionWrapper {
    pub peer_id: PeerId,
    pub peer_connection: Arc<RTCPeerConnection>,
}

impl ConnectionWrapper {
    /// Инициализация нового WebRTC соединения.
    /// event_tx — канал, в который транспорт будет "выплевывать" события для главного цикла Room.
    pub async fn new(
        peer_id: PeerId,
        config: TransportConfig,
        event_tx: mpsc::Sender<TransportEvent>,
    ) -> Result<Self> {
        // 1. Настройка MediaEngine (регистрация кодеков, даже если используем только DataChannel)
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        // 2. Регистрация интерцепторов (метрики, RTCP отчеты)
        let registry = register_default_interceptors(Registry::new(), &mut m)?;

        // 3. Создание API объекта
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        // 4. Конфигурация ICE серверов (STUN/TURN)
        let rtc_config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: config.ice_servers,
                credential: String::new(),
                username: String::new(),
            }],
            ..Default::default()
        };

        // 5. Создание PeerConnection
        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

        // --- Настройка Callbacks (Замыканий) ---
        // Важно: мы клонируем event_tx и user_id для каждого замыкания, так как они должны быть 'static.

        // A. Мониторинг состояния соединения (Connected/Disconnected)
        let state_tx = event_tx.clone();
        let uid_state = peer_id.clone();
        peer_connection.on_peer_connection_state_change(Box::new(
            move |s: RTCPeerConnectionState| {
                let tx = state_tx.clone();
                let uid = uid_state.clone();

                Box::pin(async move {
                    info!("Peer Connection State changed for user {:?}: {:?}", uid, s);
                    match s {
                        RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Disconnected
                        | RTCPeerConnectionState::Closed => {
                            let _ = tx.send(TransportEvent::Disconnected(uid)).await;
                        }
                        _ => {}
                    }
                })
            },
        ));

        // B. Trickle ICE: отправка локальных кандидатов клиенту
        let ice_tx = event_tx.clone();
        let uid_ice = peer_id.clone();
        peer_connection.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            let tx = ice_tx.clone();
            let uid = uid_ice.clone();

            Box::pin(async move {
                let Some(candidate) = c else { return };
                let Ok(json_candidate) = candidate.to_json() else {
                    return;
                };
                let Ok(str_candidate) = serde_json::to_string(&json_candidate) else {
                    return;
                };
                let _ = tx
                    .send(TransportEvent::CandidateGenerated(uid, str_candidate))
                    .await;
            })
        }));

        // C. Обработка входящего DataChannel (клиент инициирует создание канала)
        let dc_tx = event_tx.clone();
        let uid_dc = peer_id.clone();
        peer_connection.on_data_channel(Box::new(move |dс: Arc<RTCDataChannel>| {
            let tx = dc_tx.clone();
            let uid = uid_dc.clone();

            Box::pin(async move {
                debug!(
                    "New DataChannel '{:?}' created for user {:?}",
                    dс.label(),
                    uid
                );

                // Нам нужно дождаться on_open, чтобы канал был готов к записи
                let dс_on_open = dс.clone();
                let tx_open = tx.clone();
                let uid_open = uid.clone();
                dс.on_open(Box::new(move || {
                    let tx = tx_open.clone();
                    let uid = uid_open.clone();
                    let channel_ready = dс_on_open.clone(); // Клонируем арк, чтобы передать вовне

                    Box::pin(async move {
                        info!("DataChannel open and ready for user {:?}", uid);
                        // ТУТ ИСПРАВЛЕНИЕ: Передаем сам канал в Room
                        let _ = tx
                            .send(TransportEvent::DataChannelReady(uid, channel_ready))
                            .await;
                    })
                }));

                // Обработка входящих сообщений
                let tx_msg = tx.clone();
                let uid_msg = uid.clone();
                dс.on_message(Box::new(move |msg: DataChannelMessage| {
                    let tx = tx_msg.clone();
                    let uid = uid_msg.clone();
                    Box::pin(async move {
                        let bytes = Bytes::from(msg.data.to_vec());
                        let _ = tx.send(TransportEvent::Message(uid, bytes)).await;
                    })
                }));
            })
        }));

        Ok(Self {
            peer_id,
            peer_connection,
        })
    }

    /// Применить удаленный SDP Offer (полученный от клиента)
    pub async fn set_remote_description(&self, sdp: String) -> Result<()> {
        let desc =
            webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(sdp)?;
        self.peer_connection.set_remote_description(desc).await?;
        Ok(())
    }

    /// Создать локальный SDP Answer и установить его как LocalDescription
    pub async fn create_answer(&self) -> Result<String> {
        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection
            .set_local_description(answer.clone())
            .await?;
        Ok(answer.sdp)
    }

    /// Добавить удаленного ICE-кандидата (Trickle ICE)
    pub async fn add_ice_candidate(&self, candidate_json: String) -> Result<()> {
        let candidate: webrtc::ice_transport::ice_candidate::RTCIceCandidateInit =
            serde_json::from_str(&candidate_json).context("Failed to parse ICE candidate JSON")?;
        self.peer_connection.add_ice_candidate(candidate).await?;
        Ok(())
    }

    /// Закрыть WebRTC соединение
    pub async fn close(&self) -> Result<()> {
        self.peer_connection.close().await?;
        Ok(())
    }
}
