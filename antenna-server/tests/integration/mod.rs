pub mod connection_tests;
pub mod messaging_tests;
pub mod multi_peer_tests;

use tokio::sync::mpsc;
use tracing::Level;

use antenna_server::{Room, RoomCommand};

use crate::utils::{MockSignalingOutput, SignalMessage, TestRoomBehavior};

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_test_writer()
        .try_init();
}

pub fn create_test_room() -> (
    mpsc::Sender<RoomCommand>,
    mpsc::UnboundedReceiver<SignalMessage>,
    TestRoomBehavior,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<RoomCommand>(100);
    let (signaling, signal_rx) = MockSignalingOutput::new();
    let behavior = TestRoomBehavior::new();

    let room = Room::new(Box::new(behavior.clone()), cmd_rx, Box::new(signaling));

    tokio::spawn(async move {
        room.run().await;
    });

    (cmd_tx, signal_rx, behavior)
}
