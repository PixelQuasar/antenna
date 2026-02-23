use crate::{AppState, BehaviorFactory, RoomBehavior, RoomManager, SignalingService};
use antenna_core::IceServerConfig;
use std::sync::Arc;

pub struct AntennaServer {
    ice_servers: Vec<IceServerConfig>,
}

impl AntennaServer {
    pub fn new() -> Self {
        Self {
            ice_servers: Vec::new(),
        }
    }

    pub fn with_ice_server(
        mut self,
        url: impl Into<String>,
        username: Option<String>,
        credential: Option<String>,
    ) -> Self {
        self.ice_servers.push(IceServerConfig {
            urls: vec![url.into()],
            username,
            credential,
        });
        self
    }

    pub fn build<R: RoomBehavior + Default>(self) -> Arc<AppState> {
        let signaling = SignalingService::new(self.ice_servers);
        let signaling_arc = Arc::new(signaling.clone());

        let factory: BehaviorFactory = Arc::new(|| Box::new(R::default()) as Box<dyn RoomBehavior>);
        let room_manager = RoomManager::new(factory, signaling_arc);

        Arc::new(AppState {
            signaling,
            room_manager,
        })
    }
}

impl Default for AntennaServer {
    fn default() -> Self {
        Self::new()
    }
}
