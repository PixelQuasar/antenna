pub mod connection_tests;
pub mod messaging_tests;
pub mod multi_peer_tests;

use tokio::sync::mpsc;
use tracing::Level;

use antenna_server::{Room, RoomCommand};

use crate::utils::{MockSignalingOutput, TestRoomBehavior};
use antenna_core::SignalMessage;

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_test_writer()
        .try_init();
}

pub fn create_test_room() -> (
    mpsc::Sender<RoomCommand>,
    (MockSignalingOutput, mpsc::UnboundedReceiver<SignalMessage>),
    TestRoomBehavior,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<RoomCommand>(100);
    let (signaling, signal_rx) = MockSignalingOutput::new();
    let behavior = TestRoomBehavior::new();

    let room = Room::new(
        Box::new(behavior.clone()),
        cmd_rx,
        signaling.service.clone(),
    );

    tokio::spawn(async move {
        room.run().await;
    });

    (cmd_tx, (signaling, signal_rx), behavior)
}
