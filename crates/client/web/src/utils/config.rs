use antenna_client_shared::IceServerConfig;
use wasm_bindgen::prelude::*;

pub(crate) fn build_rtc_config(servers: &[IceServerConfig]) -> web_sys::RtcConfiguration {
    let config = web_sys::RtcConfiguration::new();
    let arr = js_sys::Array::new();
    for server in servers {
        arr.push(&ice_server_to_js(server));
    }
    config.set_ice_servers(&arr);
    config
}

fn ice_server_to_js(server: &IceServerConfig) -> web_sys::RtcIceServer {
    let ice = web_sys::RtcIceServer::new();
    let urls = js_sys::Array::new();
    for url in &server.urls {
        urls.push(&JsValue::from_str(url));
    }
    ice.set_urls(&urls);
    if let Some(username) = &server.username {
        ice.set_username(username);
    }
    if let Some(credential) = &server.credential {
        ice.set_credential(credential);
    }
    ice
}
