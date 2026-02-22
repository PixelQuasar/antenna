use crate::room::context::RoomContext;
use crate::room::room_behavior::RoomBehavior;
use crate::room::room_command::RoomCommand;
use crate::signaling::SignalingOutput;
use crate::transport::{ConnectionWrapper, TransportConfig, TransportEvent};
use antenna_core::PeerId;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use webrtc::data_channel::RTCDataChannel;

pub struct Room {
    behavior: Box<dyn RoomBehavior>,
    peers_data: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>,
    transports: HashMap<PeerId, ConnectionWrapper>,
    command_rx: mpsc::Receiver<RoomCommand>,
    transport_rx: mpsc::Receiver<TransportEvent>,
    transport_tx: mpsc::Sender<TransportEvent>,
    signaling: Arc<dyn SignalingOutput>,
    transport_config: TransportConfig,
}

impl Room {
    pub fn new(
        behavior: Box<dyn RoomBehavior>,
        command_rx: mpsc::Receiver<RoomCommand>,
        signaling: Arc<dyn SignalingOutput>,
    ) -> Self {
        let (transport_tx, transport_rx) = mpsc::channel(256);

        Self {
            behavior,
            peers_data: Arc::new(DashMap::new()),
            transports: HashMap::new(),
            command_rx,
            transport_rx,
            transport_tx,
            signaling,
            transport_config: TransportConfig::default(),
        }
    }

    pub async fn run(mut self) {
        info!("Room event loop started");

        loop {
            let ctx = RoomContext::new(self.peers_data.clone());

            tokio::select! {
            cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(c) => self.handle_command(c).await,
                        None => {
                            info!("Command channel closed. Shutting down room.");
                            break;
                        }
                    }
                }

                evt = self.transport_rx.recv() => {
                    match evt {
                        Some(e) => self.handle_transport_event(e, &ctx).await,
                        None => {
                            warn!("Transport channel closed unexpectedly");
                            break;
                        }
                    }
                }
            }
        }

        info!("Room event loop finished");
    }

    async fn handle_command(&mut self, cmd: RoomCommand) {
        match cmd {
            RoomCommand::JoinRequest { peer_id, offer } => {
                info!("Processing JoinRequest for user {:?}", peer_id);

                if self.transports.contains_key(&peer_id) {
                    self.remove_peer(&peer_id).await;
                }

                let transport_res = ConnectionWrapper::new(
                    peer_id.clone(),
                    self.transport_config.clone(),
                    self.transport_tx.clone(),
                )
                .await;

                match transport_res {
                    Ok(transport) => {
                        if let Err(e) = transport.set_remote_description(offer).await {
                            error!("SDP error for {:?}: {:?}", peer_id, e);
                            return;
                        }

                        match transport.create_answer().await {
                            Ok(answer_sdp) => {
                                self.transports.insert(peer_id.clone(), transport);
                                self.signaling.send_answer(peer_id, answer_sdp).await;
                            }
                            Err(e) => error!("Failed to create answer for {:?}: {:?}", peer_id, e),
                        }
                    }
                    Err(e) => error!("Failed to create transport for {:?}: {:?}", peer_id, e),
                }
            }

            RoomCommand::IceCandidate { peer_id, candidate } => {
                let Some(transport) = self.transports.get(&peer_id) else {
                    return;
                };
                let Err(e) = transport.add_ice_candidate(candidate).await else {
                    return;
                };
                warn!("Failed to add ICE candidate for {:?}: {:?}", peer_id, e);
            }

            RoomCommand::Disconnect { peer_id } => {
                self.remove_peer_with_notify(&peer_id, &RoomContext::new(self.peers_data.clone()))
                    .await;
            }
        }
    }

    async fn handle_transport_event(&mut self, event: TransportEvent, ctx: &RoomContext) {
        match event {
            TransportEvent::DataChannelReady(peer_id, channel) => {
                info!("User {:?} fully joined (DataChannel ready).", peer_id);

                self.peers_data.insert(peer_id.clone(), channel);

                self.behavior.on_join(ctx, peer_id).await;
            }

            TransportEvent::Message(peer_id, data) => {
                self.behavior.on_message(ctx, peer_id, data).await;
            }

            TransportEvent::Disconnected(peer_id) => {
                info!("Transport disconnected for {:?}", peer_id);
                self.remove_peer_with_notify(&peer_id, ctx).await;
            }

            TransportEvent::CandidateGenerated(peer_id, candidate_json) => {
                self.signaling.send_ice(peer_id, candidate_json).await;
            }
        }
    }

    async fn remove_peer_with_notify(&mut self, peer_id: &PeerId, ctx: &RoomContext) {
        let was_active = self.peers_data.contains_key(peer_id);

        self.remove_peer(peer_id).await;

        if was_active {
            self.behavior.on_leave(ctx, peer_id.clone()).await;
        }
    }

    async fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers_data.remove(peer_id);

        let Some(transport) = self.transports.remove(peer_id) else {
            return;
        };
        let _ = transport.close().await;
    }
}
