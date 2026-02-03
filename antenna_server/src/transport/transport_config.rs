/// Конфигурация для WebRTC
#[derive(Clone)]
pub struct TransportConfig {
    pub ice_servers: Vec<String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec!["stun:stun.l.google.com:19302".to_owned()],
        }
    }
}
