use crate::room::{Room, RoomBehavior, RoomCommand};
use crate::signaling::SignalingOutput;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
    behavior_factory: Arc<dyn Fn() -> Box<dyn RoomBehavior> + Send + Sync>,
    signaling: Arc<dyn SignalingOutput + Send + Sync>,
}

impl RoomManager {
    pub fn new<F>(behavior_factory: F, signaling: Arc<dyn SignalingOutput + Send + Sync>) -> Self
    where
        F: Fn() -> Box<dyn RoomBehavior> + Send + Sync + 'static,
    {
        Self {
            rooms: Arc::new(DashMap::new()),
            behavior_factory: Arc::new(behavior_factory),
            signaling,
        }
    }

    pub fn get_room_sender(&self, room_id: &str) -> mpsc::Sender<RoomCommand> {
        if let Some(sender) = self.rooms.get(room_id) {
            return sender.clone();
        }

        info!("Creating new room: {}", room_id);
        let (tx, rx) = mpsc::channel(100);
        let behavior = (self.behavior_factory)();

        let room = Room::new(behavior, rx, self.signaling.clone());
        tokio::spawn(room.run());

        self.rooms.insert(room_id.to_string(), tx.clone());
        tx
    }
}
