use wasm_bindgen::prelude::*;
use web_sys;

const GOOGLE_STUN: &str = "stun:stun.l.google.com:19302";

#[wasm_bindgen]
#[derive(Clone)]
pub struct IceServerConfig {
    urls: Vec<String>,
    username: Option<String>,
    credential: Option<String>,
}

#[wasm_bindgen]
impl IceServerConfig {
    #[wasm_bindgen(constructor)]
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
}

impl IceServerConfig {
    pub fn default_stun() -> Vec<Self> {
        vec![Self {
            urls: vec![GOOGLE_STUN.into()],
            username: None,
            credential: None,
        }]
    }

    pub(crate) fn to_rtc_ice_server(&self) -> web_sys::RtcIceServer {
        let ice = web_sys::RtcIceServer::new();

        let urls = js_sys::Array::new();
        for url in &self.urls {
            urls.push(&JsValue::from_str(url));
        }
        ice.set_urls(&urls);

        if let Some(username) = &self.username {
            ice.set_username(username);
        }
        if let Some(credential) = &self.credential {
            ice.set_credential(credential);
        }

        ice
    }

    pub(crate) fn build_rtc_config(servers: &[Self]) -> web_sys::RtcConfiguration {
        let config = web_sys::RtcConfiguration::new();
        let arr = js_sys::Array::new();

        for server in servers {
            arr.push(&server.to_rtc_ice_server());
        }

        config.set_ice_servers(&arr);
        config
    }
}
