use antenna_core::model::PeerId;
use bytes::Bytes;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;

/// События, которые транспорт генерирует для логики Комнаты (Room).
pub enum TransportEvent {
    /// DataChannel успешно открыт и готов к передаче данных.
    /// Передаем сам канал, чтобы RoomContext мог его сохранить.
    DataChannelReady(PeerId, Arc<RTCDataChannel>),

    /// Соединение с пиром разорвано.
    Disconnected(PeerId),

    /// Получено бинарное сообщение от пира.
    Message(PeerId, Bytes),

    /// Сгенерирован локальный ICE-кандидат, его нужно отправить клиенту (через Signalling).
    CandidateGenerated(PeerId, String),
}
