use crate::SignalingService;
use crate::room::context::RoomContext;
use crate::room::room_behavior::RoomBehavior;
use crate::room::room_command::RoomCommand;
use crate::transport::{ConnectionWrapper, TransportConfig, TransportEvent};
use antenna_core::{PeerId, SignalMessage};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use webrtc::data_channel::RTCDataChannel;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

pub type BehaviorFactory = Arc<dyn Fn() -> Box<dyn RoomBehavior> + Send + Sync>;

/// Track handling sender to provide SFU
struct SFUTrackSender {
    tx: broadcast::Sender<Packet>,
    codec: RTCRtpCodecCapability,
    stream_id: String,
}

/// Central actor unit of antenna state. Handles specific group of peer connections and all logic around them.
pub struct Room {
    /// User-implemented room logic
    behavior: Box<dyn RoomBehavior>,

    /// Map of active room data channels, passed to room context in room loop
    peers_data: Arc<DashMap<PeerId, Arc<RTCDataChannel>>>,

    /// Map of active webrtc connections of room participants
    transports: HashMap<PeerId, ConnectionWrapper>,

    /// Room control signaling command receiver: or can be said, room central input
    command_rx: mpsc::Receiver<RoomCommand>,

    /// Signaling sending service instance
    signaling_service: Arc<SignalingService>,

    /// Room <=> WebRTC transport channel receiver: used to process room network events
    transport_rx: mpsc::Receiver<TransportEvent>,

    /// Room <=> WebRTC transport channel sender: used to send network events
    transport_tx: mpsc::Sender<TransportEvent>,

    /// Config to create WebRTC connections, contains settings like ICE servers
    transport_config: TransportConfig,

    /// map of track senders to provide SFU mechanism for media tracks
    track_senders: HashMap<String, SFUTrackSender>,
}

impl Room {
    pub fn new(
        behavior: Box<dyn RoomBehavior>,
        command_rx: mpsc::Receiver<RoomCommand>,
        signaling_service: Arc<SignalingService>,
    ) -> Self {
        let (transport_tx, transport_rx) = mpsc::channel(256);

        Self {
            behavior,
            peers_data: Arc::new(DashMap::new()),
            transports: HashMap::new(),
            command_rx,
            transport_rx,
            transport_tx,
            signaling_service,
            transport_config: TransportConfig::default(),
            track_senders: HashMap::new(),
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

                        for (track_id, sfu_sender) in &self.track_senders {
                            let local_track = Arc::new(TrackLocalStaticRTP::new(
                                sfu_sender.codec.clone(),
                                track_id.clone(),
                                sfu_sender.stream_id.clone(),
                            ));

                            if transport.add_track(local_track.clone()).await.is_ok() {
                                let mut rx = sfu_sender.tx.subscribe();
                                tokio::spawn(async move {
                                    while let Ok(packet) = rx.recv().await {
                                        let _ = local_track.write_rtp(&packet).await;
                                    }
                                });
                            }
                        }

                        match transport.create_answer().await {
                            Ok(answer_sdp) => {
                                self.transports.insert(peer_id.clone(), transport);
                                self.signaling_service.send_signal(
                                    peer_id,
                                    SignalMessage::Answer { sdp: answer_sdp },
                                );
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
                self.signaling_service.send_signal(
                    peer_id,
                    SignalMessage::IceCandidate {
                        candidate: candidate_json,
                    },
                );
            }

            TransportEvent::Track(peer_id, track) => {
                info!("Track received from {:?}: id={}", peer_id, track.id());
                let (tx, _) = broadcast::channel(100);
                self.track_senders.insert(
                    track.id().to_string(),
                    SFUTrackSender {
                        tx: tx.clone(),
                        codec: track.codec().capability.clone(),
                        stream_id: track.stream_id().to_string(),
                    },
                );

                let track_clone = track.clone();
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    while let Ok((packet, _)) = track_clone.read_rtp().await {
                        let _ = tx_clone.send(packet);
                    }
                });

                for (other_peer_id, transport) in &self.transports {
                    if *other_peer_id != peer_id {
                        let local_track = Arc::new(TrackLocalStaticRTP::new(
                            track.codec().capability.clone(),
                            track.id(),
                            track.stream_id(),
                        ));

                        if transport.add_track(local_track.clone()).await.is_ok() {
                            let mut rx = tx.subscribe();
                            tokio::spawn(async move {
                                while let Ok(packet) = rx.recv().await {
                                    let _ = local_track.write_rtp(&packet).await;
                                }
                            });
                        }
                    }
                }
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
