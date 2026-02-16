use anyhow::{Context, Result};
use tokio::sync::mpsc;

use antenna_core::PeerId;
use antenna_server::RoomCommand;

use super::mock_signaling::SignalMessage;
use super::test_client::TestClient;

/// Timeout for signal exchange operations (ms).
pub const SIGNAL_TIMEOUT_MS: u64 = 5000;

/// Timeout for ICE gathering (ms).
pub const ICE_GATHERING_TIMEOUT_MS: u64 = 3000;

/// Timeout for connection establishment (ms).
pub const CONNECTION_TIMEOUT_MS: u64 = 10000;

/// Timeout for data channel opening (ms).
pub const DATA_CHANNEL_TIMEOUT_MS: u64 = 5000;

/// Helper to perform full signaling exchange between a TestClient and Room.
///
/// This function:
/// 1. Creates an offer from the client
/// 2. Sends JoinRequest to the room
/// 3. Waits for the answer from the room
/// 4. Applies the answer to the client
/// 5. Exchanges ICE candidates
///
/// # Arguments
///
/// * `client` - The test client
/// * `room_cmd_tx` - Channel to send commands to the room
/// * `signal_rx` - Channel to receive signals from the room
///
/// # Returns
///
/// Ok(()) if signaling completed successfully.
pub async fn perform_signaling(
    client: &TestClient,
    room_cmd_tx: &mpsc::Sender<RoomCommand>,
    signal_rx: &mut mpsc::UnboundedReceiver<SignalMessage>,
) -> Result<()> {
    let peer_id = client.peer_id.clone();

    // 1. Create offer
    let offer = client
        .create_offer()
        .await
        .context("Failed to create offer")?;
    tracing::debug!("[SignalHelper] Created offer for {:?}", peer_id);

    // 2. Send JoinRequest to room
    room_cmd_tx
        .send(RoomCommand::JoinRequest {
            peer_id: peer_id.clone(),
            offer,
        })
        .await
        .context("Failed to send JoinRequest")?;
    tracing::debug!("[SignalHelper] Sent JoinRequest for {:?}", peer_id);

    // 3. Wait for answer from room
    let answer_sdp = wait_for_answer(signal_rx, &peer_id, SIGNAL_TIMEOUT_MS)
        .await
        .context("Failed to receive answer")?;
    tracing::debug!("[SignalHelper] Received answer for {:?}", peer_id);

    // 4. Apply answer to client
    client
        .set_remote_answer(answer_sdp)
        .await
        .context("Failed to set remote answer")?;
    tracing::debug!("[SignalHelper] Applied answer for {:?}", peer_id);

    // 5. Exchange ICE candidates (in background)
    // The room will send ICE candidates through signal_rx
    // We need to forward them to the client
    exchange_ice_candidates(client, room_cmd_tx, signal_rx).await?;

    Ok(())
}

/// Wait for an SDP answer for a specific peer.
async fn wait_for_answer(
    signal_rx: &mut mpsc::UnboundedReceiver<SignalMessage>,
    peer_id: &PeerId,
    timeout_ms: u64,
) -> Result<String> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    loop {
        let recv_timeout =
            tokio::time::timeout(std::time::Duration::from_millis(100), signal_rx.recv());

        match recv_timeout.await {
            Ok(Some(SignalMessage::Answer { peer_id: id, sdp })) if &id == peer_id => {
                return Ok(sdp);
            }
            Ok(Some(SignalMessage::Ice { .. })) => {
                // ICE candidates may arrive before answer, ignore for now
                continue;
            }
            Ok(Some(_)) => continue,
            Ok(None) => anyhow::bail!("Signal channel closed"),
            Err(_) => {
                // Timeout on recv, check overall timeout
                if start.elapsed() > timeout {
                    anyhow::bail!("Timeout waiting for answer");
                }
            }
        }
    }
}

/// Exchange ICE candidates between client and room.
///
/// This runs for a limited time to allow ICE candidates to be exchanged.
async fn exchange_ice_candidates(
    client: &TestClient,
    room_cmd_tx: &mpsc::Sender<RoomCommand>,
    signal_rx: &mut mpsc::UnboundedReceiver<SignalMessage>,
) -> Result<()> {
    let peer_id = client.peer_id.clone();
    let exchange_duration = std::time::Duration::from_millis(ICE_GATHERING_TIMEOUT_MS);
    let start = std::time::Instant::now();

    // Spawn a task to send client's ICE candidates to the room
    let room_cmd_tx_clone = room_cmd_tx.clone();
    let peer_id_clone = peer_id.clone();
    let client_candidates = client
        .gather_ice_candidates(ICE_GATHERING_TIMEOUT_MS)
        .await?;

    for candidate in client_candidates {
        let _ = room_cmd_tx_clone
            .send(RoomCommand::IceCandidate {
                peer_id: peer_id_clone.clone(),
                candidate,
            })
            .await;
    }

    // Process incoming ICE candidates from the room
    while start.elapsed() < exchange_duration {
        let recv_timeout =
            tokio::time::timeout(std::time::Duration::from_millis(100), signal_rx.recv());

        match recv_timeout.await {
            Ok(Some(SignalMessage::Ice {
                peer_id: id,
                candidate,
            })) if id == peer_id => {
                if let Err(e) = client.add_ice_candidate(candidate).await {
                    tracing::warn!("[SignalHelper] Failed to add ICE candidate: {}", e);
                }
            }
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => continue, // Timeout, continue loop
        }
    }

    Ok(())
}

/// Simplified signaling for tests that don't need ICE trickling.
///
/// Uses ICE gathering complete before exchanging SDP.
pub async fn perform_signaling_with_gathered_ice(
    client: &TestClient,
    room_cmd_tx: &mpsc::Sender<RoomCommand>,
    signal_rx: &mut mpsc::UnboundedReceiver<SignalMessage>,
) -> Result<()> {
    let peer_id = client.peer_id.clone();

    // 1. Create offer
    let offer = client.create_offer().await?;

    // 2. Wait for ICE gathering to complete
    let _ = client
        .gather_ice_candidates(ICE_GATHERING_TIMEOUT_MS)
        .await?;

    // 3. Get the complete local description (with ICE candidates)
    // Note: The offer SDP already contains candidates after gathering

    // 4. Send JoinRequest
    room_cmd_tx
        .send(RoomCommand::JoinRequest {
            peer_id: peer_id.clone(),
            offer,
        })
        .await?;

    // 5. Wait for answer
    let answer_sdp = wait_for_answer(signal_rx, &peer_id, SIGNAL_TIMEOUT_MS).await?;

    // 6. Apply answer
    client.set_remote_answer(answer_sdp).await?;

    // 7. Process any remaining ICE candidates from server
    let ice_timeout = std::time::Duration::from_millis(1000);
    let start = std::time::Instant::now();

    while start.elapsed() < ice_timeout {
        let recv_timeout =
            tokio::time::timeout(std::time::Duration::from_millis(100), signal_rx.recv());

        match recv_timeout.await {
            Ok(Some(SignalMessage::Ice {
                peer_id: id,
                candidate,
            })) if id == peer_id => {
                let _ = client.add_ice_candidate(candidate).await;
            }
            _ => break,
        }
    }

    Ok(())
}

/// Wait for the client to be fully connected (connection + data channel).
pub async fn wait_for_client_ready(client: &TestClient) -> Result<()> {
    // Wait for WebRTC connection
    client
        .wait_for_connection(CONNECTION_TIMEOUT_MS)
        .await
        .context("Connection not established")?;

    // Wait for data channel
    client
        .wait_for_data_channel(DATA_CHANNEL_TIMEOUT_MS)
        .await
        .context("Data channel not opened")?;

    Ok(())
}

/// Helper struct to manage a test scenario with multiple clients.
pub struct TestScenario {
    pub room_cmd_tx: mpsc::Sender<RoomCommand>,
    pub signal_rx: mpsc::UnboundedReceiver<SignalMessage>,
}

impl TestScenario {
    /// Connect a client to the room and wait for it to be ready.
    pub async fn connect_client(&mut self, client: &TestClient) -> Result<()> {
        perform_signaling(client, &self.room_cmd_tx, &mut self.signal_rx).await?;
        wait_for_client_ready(client).await?;
        Ok(())
    }

    /// Disconnect a client from the room.
    pub async fn disconnect_client(&mut self, client: &TestClient) -> Result<()> {
        self.room_cmd_tx
            .send(RoomCommand::Disconnect {
                peer_id: client.peer_id.clone(),
            })
            .await?;
        client.close().await?;
        Ok(())
    }
}
