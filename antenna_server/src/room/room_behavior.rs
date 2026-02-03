use crate::room::context::RoomContext;
use antenna_core::model::PeerId;
use async_trait::async_trait;
use bytes::Bytes; // Предполагаем, что UserId определен в core

/// Треит, определяющий логику поведения комнаты.
/// Пользователь реализует этот трейт, чтобы обрабатывать события.
#[async_trait]
pub trait RoomBehavior: Send + Sync + 'static {
    /// Вызывается, когда пир успешно установил WebRTC соединение (DataChannel открыт).
    /// Здесь можно отправить приветственное сообщение или загрузить состояние игрока.
    async fn on_join(&self, ctx: &RoomContext, peer_id: PeerId) {
        // По умолчанию ничего не делаем
    }

    /// Вызывается, когда от пира приходят бинарные данные.
    /// `data` - это сырые байты. Ожидается, что пользователь сам десериализует их (Protobuf/JSON).
    async fn on_message(&self, ctx: &RoomContext, peer_id: PeerId, data: Bytes) {
        // По умолчанию ничего не делаем
    }

    /// Вызывается при разрыве соединения (тайм-аут или явное закрытие).
    async fn on_leave(&self, ctx: &RoomContext, peer_id: PeerId) {
        // По умолчанию ничего не делаем
    }
}
