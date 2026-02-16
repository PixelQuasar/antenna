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
pub async fn perform_signaling(
    client: &TestClient,
    room_cmd_tx: &mpsc::Sender<RoomCommand>,
    signal_rx: &mut mpsc::UnboundedReceiver<SignalMessage>,
) -> Result<()> {
    let peer_id = client.peer_id.clone();

    let offer = client
        .create_offer()
        .await
        .context("Failed to create offer")?;
    tracing::debug!("[SignalHelper] Created offer for {:?}", peer_id);

    room_cmd_tx
        .send(RoomCommand::JoinRequest {
            peer_id: peer_id.clone(),
            offer,
        })
        .await
        .context("Failed to send JoinRequest")?;
    tracing::debug!("[SignalHelper] Sent JoinRequest for {:?}", peer_id);

    let answer_sdp = wait_for_answer(signal_rx, &peer_id, SIGNAL_TIMEOUT_MS)
        .await
        .context("Failed to receive answer")?;
    tracing::debug!("[SignalHelper] Received answer for {:?}", peer_id);

    client
        .set_remote_answer(answer_sdp)
        .await
        .context("Failed to set remote answer")?;
    tracing::debug!("[SignalHelper] Applied answer for {:?}", peer_id);

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
                continue;
            }
            Ok(Some(_)) => continue,
            Ok(None) => anyhow::bail!("Signal channel closed"),
            Err(_) => {
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
            Err(_) => continue,
        }
    }

    Ok(())
}

/// Wait for the client to be fully connected (connection + data channel).
pub async fn wait_for_client_ready(client: &TestClient) -> Result<()> {
    client
        .wait_for_connection(CONNECTION_TIMEOUT_MS)
        .await
        .context("Connection not established")?;

    client
        .wait_for_data_channel(DATA_CHANNEL_TIMEOUT_MS)
        .await
        .context("Data channel not opened")?;

    Ok(())
}
