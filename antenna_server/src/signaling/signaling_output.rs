use antenna_core::PeerId;
use async_trait::async_trait;

/// Трейт, который должна реализовать внешняя система (WebSocket сервер),
/// чтобы комната могла отправлять ответы клиентам (SDP Answer, ICE).
#[async_trait]
pub trait SignalingOutput: Send + Sync {
    /// Отправить SDP Answer конкретному пользователю.
    async fn send_answer(&self, peer_id: PeerId, sdp: String);

    /// Отправить ICE кандидата конкретному пользователю.
    async fn send_ice(&self, peer_id: PeerId, candidate: String);
}
