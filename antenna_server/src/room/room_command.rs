use antenna_core::PeerId;

/// Команды, поступающие в комнату от сигнального сервера (WebSocket/HTTP).
#[derive(Debug)]
pub enum RoomCommand {
    /// Запрос на подключение: новый пользователь прислал SDP Offer.
    JoinRequest { peer_id: PeerId, offer: String },

    /// ICE Candidate от клиента (для пробития NAT).
    IceCandidate { peer_id: PeerId, candidate: String },

    /// Сигнал о разрыве WebSocket соединения.
    Disconnect { peer_id: PeerId },
}
