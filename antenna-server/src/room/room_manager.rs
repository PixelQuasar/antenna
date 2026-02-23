use crate::BehaviorFactory;
use crate::room::{Room, RoomCommand};
use crate::signaling::SignalingSender;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
    behavior_factory: BehaviorFactory,
    signaling_sender: Arc<dyn SignalingSender + Send + Sync>,
}

impl RoomManager {
    pub fn new(
        behavior_factory: BehaviorFactory,
        signaling_sender: Arc<dyn SignalingSender + Send + Sync>,
    ) -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            behavior_factory,
            signaling_sender,
        }
    }

    pub fn get_room_sender(&self, room_id: &str) -> mpsc::Sender<RoomCommand> {
        if let Some(sender) = self.rooms.get(room_id) {
            return sender.clone();
        }

        info!("Creating new room: {}", room_id);
        let (tx, rx) = mpsc::channel(256);
        let behavior = (self.behavior_factory)();

        let room = Room::new(behavior, rx, self.signaling_sender.clone());
        tokio::spawn(room.run());

        self.rooms.insert(room_id.to_string(), tx.clone());
        tx
    }
}
