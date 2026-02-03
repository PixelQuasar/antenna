use antenna_core::PeerId;
use bytes::Bytes;
use dashmap::DashMap;
use std::ops::Deref;
use std::sync::Arc;
use tracing::error;
use webrtc::data_channel::RTCDataChannel;

/// Контекст комнаты, предоставляющий методы для взаимодействия с подключенными пирами.
/// Эту структуру безопасно клонировать и передавать между потоками.
#[derive(Clone)]
pub struct RoomContext {
    peers: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>,
}

impl RoomContext {
    pub(crate) fn new(peers: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>) -> Self {
        Self { peers }
    }

    /// Отправить бинарное сообщение конкретному пользователю.
    pub async fn send(&self, peer_id: &PeerId, data: Bytes) {
        if let Some(peer) = self.peers.get(peer_id) {
            // DataChannel::send ожидает Bytes и является асинхронным
            if let Err(e) = peer.send(&data).await {
                error!("Failed to send message to user {:?}: {}", peer_id, e);
            }
        } else {
            // Пользователь мог отключиться в момент обработки
            error!(
                "Attempted to send message to disconnected user {:?}",
                peer_id
            );
        }
    }

    /// Разослать сообщение всем подключенным пользователям (кроме исключений, если нужно).
    pub async fn broadcast(&self, data: Bytes) {
        // Мы итерируемся по всем пирам (dashmap не блокируется на запись во время чтения)
        // Но send - асинхронный, поэтому мы не можем вызвать его прямо внутри итератора dashmap
        // (так как итератор держит guard).

        // 1. Соберем список каналов, чтобы отпустить блокировку карты
        let mut channels = Vec::new();
        for entry in self.peers.iter() {
            channels.push(entry.value().clone());
        }

        // 2. Отправим всем (можно параллельно через spawn, но здесь сделаем последовательно для простоты)
        for channel in channels {
            let data_clone = data.clone();
            tokio::spawn(async move {
                if let Err(e) = channel.send(&data_clone).await {
                    error!("Broadcast error: {}", e);
                }
            });
        }
    }

    pub fn list_users(&self) -> Vec<PeerId> {
        self.peers.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn contains_user(&self, peer_id: &PeerId) -> bool {
        self.peers.contains_key(peer_id)
    }
}
