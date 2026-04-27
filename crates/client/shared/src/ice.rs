const GOOGLE_STUN: &str = "stun:stun.l.google.com:19302";

#[derive(Clone)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

impl IceServerConfig {
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            username: None,
            credential: None,
        }
    }

    pub fn with_credentials(urls: Vec<String>, username: String, credential: String) -> Self {
        Self {
            urls,
            username: Some(username),
            credential: Some(credential),
        }
    }

    pub fn default_stun() -> Vec<Self> {
        vec![Self::new(vec![GOOGLE_STUN.into()])]
    }
}
