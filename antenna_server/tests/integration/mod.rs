//! Integration tests for antenna_server.
//!
//! Tests are organized by functionality:
//! - `connection_tests` - peer connection and disconnection
//! - `messaging_tests` - data channel messaging
//! - `multi_peer_tests` - multiple peers in a room

pub mod connection_tests;
pub mod messaging_tests;
pub mod multi_peer_tests;

use tokio::sync::mpsc;
use tracing::Level;

use antenna_server::{Room, RoomCommand};

use crate::utils::{MockSignalingOutput, SignalMessage, TestRoomBehavior};

/// Initialize tracing for tests (call once per test).
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_test_writer()
        .try_init();
}

/// Create a room with test behavior and signaling.
///
/// Returns (room_cmd_tx, signal_rx, behavior) for test control.
pub fn create_test_room() -> (
    mpsc::Sender<RoomCommand>,
    mpsc::UnboundedReceiver<SignalMessage>,
    TestRoomBehavior,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<RoomCommand>(100);
    let (signaling, signal_rx) = MockSignalingOutput::new();
    let behavior = TestRoomBehavior::new();

    let room = Room::new(Box::new(behavior.clone()), cmd_rx, Box::new(signaling));

    // Spawn room event loop
    tokio::spawn(async move {
        room.run().await;
    });

    (cmd_tx, signal_rx, behavior)
}
