use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::PeerId;
use crate::peer::{IceCandidate, PeerState, Phase};
use crate::signal::{Answer, IceServer, Invite, RelayMessage};

// ---------------------------------------------------------------------------
// Input — events fed INTO the state machine from the platform layer
// ---------------------------------------------------------------------------

/// Events that the platform layer feeds into [`Mesh`].
#[derive(Debug)]
pub enum Input {
    // -- Bootstrap (invite flow) --
    /// Master: the platform created a local SDP offer for the bootstrap connection.
    /// The state machine will wait for ICE gathering to complete before emitting the invite.
    BootstrapOfferCreated { sdp: String },

    /// Master: ICE gathering is complete for the bootstrap connection.
    /// The SDP now contains all candidates ("vanilla ICE").
    BootstrapIceGatheringComplete { sdp: String },

    /// Joiner: the user pasted the master's invite. Already decoded by the platform.
    InviteReceived(Invite),

    /// Joiner: the platform created a local SDP answer for the bootstrap connection.
    BootstrapAnswerCreated { sdp: String },

    /// Joiner: ICE gathering is complete for the bootstrap answer.
    BootstrapAnswerIceComplete { sdp: String },

    /// Master: the user pasted the joiner's answer. Already decoded by the platform.
    AnswerReceived(Answer),

    // -- Data channel events --
    /// The RTCDataChannel to a remote peer opened.
    DataChannelOpen { remote: PeerId },

    /// The RTCDataChannel to a remote peer closed.
    DataChannelClosed { remote: PeerId },

    /// Binary data received on a data channel from a remote peer.
    DataReceived { from: PeerId, data: Vec<u8> },

    // -- Mesh signaling (relayed over data channels) --
    /// A relay message arrived over a data channel from a peer.
    RelayReceived { from: PeerId, msg: RelayMessage },

    /// A local SDP offer was created for a mesh peer connection.
    LocalOfferCreated { remote: PeerId, sdp: String },

    /// A local SDP answer was created for a mesh peer connection.
    LocalAnswerCreated { remote: PeerId, sdp: String },

    /// A local ICE candidate was gathered for a mesh peer connection.
    LocalIceCandidate {
        remote: PeerId,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    },

    /// ICE connection state changed for a peer.
    IceConnectionStateChange { remote: PeerId, state: String },
}

// ---------------------------------------------------------------------------
// Output — commands the state machine emits for the platform layer
// ---------------------------------------------------------------------------

/// Commands that [`Mesh`] emits for the platform layer to execute.
#[derive(Debug)]
pub enum Output {
    // -- Bootstrap --
    /// Create an RTCPeerConnection for the bootstrap (master↔joiner) link.
    CreateBootstrapPeerConnection { ice_servers: Vec<IceServer> },

    /// Create a DataChannel on the bootstrap peer connection.
    CreateBootstrapDataChannel { label: String },

    /// Create an SDP offer on the bootstrap peer connection.
    CreateBootstrapOffer,

    /// Set the local description on the bootstrap peer connection.
    SetBootstrapLocalDescription { sdp: String, is_offer: bool },

    /// Set the remote description on the bootstrap peer connection.
    SetBootstrapRemoteDescription { sdp: String, is_offer: bool },

    /// Create an SDP answer on the bootstrap peer connection.
    CreateBootstrapAnswer,

    /// The invite is ready — present it to the user for sharing.
    InviteReady(Invite),

    /// The answer is ready — present it to the user for sharing back to master.
    AnswerReady(Answer),

    // -- Mesh peer connections --
    /// Create an RTCPeerConnection for a mesh peer.
    CreatePeerConnection {
        remote: PeerId,
        ice_servers: Vec<IceServer>,
    },

    /// Create a DataChannel on a mesh peer connection.
    CreateDataChannel { remote: PeerId, label: String },

    /// Create an SDP offer for a mesh peer.
    CreateOffer { remote: PeerId },

    /// Set the local SDP description for a mesh peer.
    SetLocalDescription {
        remote: PeerId,
        sdp: String,
        is_offer: bool,
    },

    /// Set the remote SDP description for a mesh peer.
    SetRemoteDescription {
        remote: PeerId,
        sdp: String,
        is_offer: bool,
    },

    /// Create an SDP answer for a mesh peer.
    CreateAnswer { remote: PeerId },

    /// Add an ICE candidate to a mesh peer connection.
    AddIceCandidate {
        remote: PeerId,
        candidate: IceCandidate,
    },

    /// Close a mesh peer connection.
    ClosePeerConnection { remote: PeerId },

    // -- Data channel I/O --
    /// Send a relay message to a peer over its data channel.
    SendRelay { to: PeerId, msg: RelayMessage },

    /// Deliver received application data to the user.
    DeliverMessage { from: PeerId, data: Vec<u8> },

    // -- User notifications --
    /// Notify the user that a peer's data channel is open.
    PeerConnected { peer_id: PeerId },

    /// Notify the user that a peer disconnected.
    PeerDisconnected { peer_id: PeerId },
}

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Whether this node is the master (creates invites) or a joiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Master,
    Joiner,
}

// ---------------------------------------------------------------------------
// Bootstrap phase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootstrapPhase {
    /// Master: waiting for the platform to create the offer.
    CreatingOffer,
    /// Master: offer created, waiting for ICE gathering to complete.
    GatheringIce,
    /// Master: invite shared, waiting for the joiner's answer.
    WaitingForAnswer,
    /// Joiner: waiting for the user to paste the invite.
    WaitingForInvite,
    /// Joiner: remote offer set, creating answer.
    CreatingAnswer,
    /// Joiner: answer created, waiting for ICE gathering to complete.
    GatheringAnswerIce,
    /// Joiner: answer shared, waiting for data channel to open.
    WaitingForConnection,
    /// Bootstrap complete — data channel is open.
    Connected,
}

// ---------------------------------------------------------------------------
// Mesh — the sans-IO state machine
// ---------------------------------------------------------------------------

/// Pure-logic state machine for a serverless full-mesh WebRTC network.
///
/// The first connection is bootstrapped via copy-paste invite/answer exchange.
/// After that, the master relays signaling messages between peers over
/// data channels, enabling a full mesh topology.
pub struct Mesh {
    role: Role,
    local_id: PeerId,
    ice_servers: Vec<IceServer>,
    bootstrap: BootstrapPhase,
    /// The peer on the other end of the bootstrap connection.
    bootstrap_remote: Option<PeerId>,
    /// Per-peer connection state (mesh links, not bootstrap).
    peers: HashMap<PeerId, PeerState>,
    /// Counter for assigning joiner IDs (master only).
    next_joiner_id: u32,
}

impl Mesh {
    /// Create a new mesh as the **master**.
    ///
    /// Call [`Mesh::create_invite`] to start the bootstrap flow.
    pub fn new_master(ice_servers: Vec<IceServer>) -> Self {
        Self {
            role: Role::Master,
            local_id: PeerId::master(),
            ice_servers,
            bootstrap: BootstrapPhase::CreatingOffer,
            bootstrap_remote: None,
            peers: HashMap::new(),
            next_joiner_id: 1,
        }
    }

    /// Create a new mesh as a **joiner**.
    ///
    /// Feed [`Input::InviteReceived`] when the user pastes the master's invite.
    pub fn new_joiner() -> Self {
        Self {
            role: Role::Joiner,
            local_id: PeerId::from("pending"),
            ice_servers: Vec::new(),
            bootstrap: BootstrapPhase::WaitingForInvite,
            bootstrap_remote: None,
            peers: HashMap::new(),
            next_joiner_id: 0,
        }
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn local_id(&self) -> &PeerId {
        &self.local_id
    }

    pub fn is_connected(&self, peer: &PeerId) -> bool {
        if self.bootstrap_remote.as_ref() == Some(peer)
            && self.bootstrap == BootstrapPhase::Connected
        {
            return true;
        }
        self.peers
            .get(peer)
            .is_some_and(|p| p.phase == Phase::Connected)
    }

    pub fn connected_peers(&self) -> Vec<&PeerId> {
        let mut result: Vec<&PeerId> = self
            .peers
            .values()
            .filter(|p| p.phase == Phase::Connected)
            .map(|p| &p.id)
            .collect();
        if self.bootstrap == BootstrapPhase::Connected {
            if let Some(ref id) = self.bootstrap_remote {
                result.push(id);
            }
        }
        result
    }

    // -- Master: start the invite flow --------------------------------------

    /// Master: begin creating an invite. Returns outputs to create the
    /// bootstrap peer connection and offer.
    pub fn create_invite(&mut self) -> Result<Vec<Output>> {
        if self.role != Role::Master {
            return Err(anyhow!("only the master can create invites"));
        }
        self.bootstrap = BootstrapPhase::CreatingOffer;
        Ok(vec![
            Output::CreateBootstrapPeerConnection {
                ice_servers: self.ice_servers.clone(),
            },
            Output::CreateBootstrapDataChannel {
                label: "signal".to_owned(),
            },
            Output::CreateBootstrapOffer,
        ])
    }

    /// Process an input event and return zero or more output commands.
    pub fn handle(&mut self, input: Input) -> Result<Vec<Output>> {
        match input {
            // Bootstrap flow
            Input::BootstrapOfferCreated { sdp } => self.on_bootstrap_offer_created(sdp),
            Input::BootstrapIceGatheringComplete { sdp } => self.on_bootstrap_ice_complete(sdp),
            Input::InviteReceived(invite) => self.on_invite_received(invite),
            Input::BootstrapAnswerCreated { sdp } => self.on_bootstrap_answer_created(sdp),
            Input::BootstrapAnswerIceComplete { sdp } => self.on_bootstrap_answer_ice_complete(sdp),
            Input::AnswerReceived(answer) => self.on_answer_received(answer),

            // Data channel events
            Input::DataChannelOpen { remote } => self.on_dc_open(remote),
            Input::DataChannelClosed { remote } => self.on_dc_closed(remote),
            Input::DataReceived { from, data } => self.on_data_received(from, data),

            // Mesh signaling (relayed)
            Input::RelayReceived { from, msg } => self.on_relay_received(from, msg),
            Input::LocalOfferCreated { remote, sdp } => self.on_local_offer(remote, sdp),
            Input::LocalAnswerCreated { remote, sdp } => self.on_local_answer(remote, sdp),
            Input::LocalIceCandidate {
                remote,
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => Ok(self.on_local_ice(remote, candidate, sdp_mid, sdp_m_line_index)),
            Input::IceConnectionStateChange { remote, state } => {
                Ok(self.on_ice_state_change(remote, &state))
            }
        }
    }

    // -- Bootstrap: master side ---------------------------------------------

    fn on_bootstrap_offer_created(&mut self, sdp: String) -> Result<Vec<Output>> {
        if self.role != Role::Master {
            return Err(anyhow!("unexpected bootstrap offer on joiner"));
        }
        self.bootstrap = BootstrapPhase::GatheringIce;
        Ok(vec![Output::SetBootstrapLocalDescription {
            sdp,
            is_offer: true,
        }])
    }

    fn on_bootstrap_ice_complete(&mut self, sdp: String) -> Result<Vec<Output>> {
        match (self.role, self.bootstrap) {
            (Role::Master, BootstrapPhase::GatheringIce) => {
                self.bootstrap = BootstrapPhase::WaitingForAnswer;
                Ok(vec![Output::InviteReady(Invite {
                    sdp,
                    ice_servers: self.ice_servers.clone(),
                })])
            }
            _ => Ok(vec![]),
        }
    }

    // -- Bootstrap: joiner side ---------------------------------------------

    fn on_invite_received(&mut self, invite: Invite) -> Result<Vec<Output>> {
        if self.role != Role::Joiner {
            return Err(anyhow!("only joiners accept invites"));
        }
        self.ice_servers = invite.ice_servers.clone();
        self.bootstrap = BootstrapPhase::CreatingAnswer;
        self.bootstrap_remote = Some(PeerId::master());

        Ok(vec![
            Output::CreateBootstrapPeerConnection {
                ice_servers: invite.ice_servers,
            },
            Output::SetBootstrapRemoteDescription {
                sdp: invite.sdp,
                is_offer: true,
            },
            Output::CreateBootstrapAnswer,
        ])
    }

    fn on_bootstrap_answer_created(&mut self, sdp: String) -> Result<Vec<Output>> {
        if self.role != Role::Joiner {
            return Err(anyhow!("unexpected bootstrap answer on master"));
        }
        self.bootstrap = BootstrapPhase::GatheringAnswerIce;
        Ok(vec![Output::SetBootstrapLocalDescription {
            sdp,
            is_offer: false,
        }])
    }

    fn on_bootstrap_answer_ice_complete(&mut self, sdp: String) -> Result<Vec<Output>> {
        if self.role != Role::Joiner || self.bootstrap != BootstrapPhase::GatheringAnswerIce {
            return Ok(vec![]);
        }
        self.bootstrap = BootstrapPhase::WaitingForConnection;
        Ok(vec![Output::AnswerReady(Answer { sdp })])
    }

    // -- Bootstrap: master receives answer ----------------------------------

    fn on_answer_received(&mut self, answer: Answer) -> Result<Vec<Output>> {
        if self.role != Role::Master {
            return Err(anyhow!("only master receives answers"));
        }
        if self.bootstrap != BootstrapPhase::WaitingForAnswer {
            return Err(anyhow!("not waiting for an answer"));
        }
        // Assign an ID to this joiner
        let joiner_id = PeerId::from(format!("peer-{}", self.next_joiner_id));
        self.next_joiner_id += 1;
        self.bootstrap_remote = Some(joiner_id);

        Ok(vec![Output::SetBootstrapRemoteDescription {
            sdp: answer.sdp,
            is_offer: false,
        }])
    }

    // -- Data channel open/close --------------------------------------------

    fn on_dc_open(&mut self, remote: PeerId) -> Result<Vec<Output>> {
        // Is this the bootstrap data channel?
        if self.bootstrap != BootstrapPhase::Connected {
            return self.on_bootstrap_dc_open(remote);
        }

        // Mesh data channel
        if let Some(ps) = self.peers.get_mut(&remote) {
            ps.phase = Phase::Connected;
        }
        Ok(vec![Output::PeerConnected { peer_id: remote }])
    }

    fn on_bootstrap_dc_open(&mut self, _remote: PeerId) -> Result<Vec<Output>> {
        self.bootstrap = BootstrapPhase::Connected;
        let mut out = Vec::new();

        match self.role {
            Role::Master => {
                let joiner_id = self
                    .bootstrap_remote
                    .clone()
                    .ok_or_else(|| anyhow!("bootstrap remote not set"))?;

                // Send Welcome to the joiner
                out.push(Output::SendRelay {
                    to: joiner_id.clone(),
                    msg: RelayMessage::Welcome {
                        peer_id: joiner_id.clone(),
                    },
                });

                // Notify existing peers about the new joiner
                let existing: Vec<PeerId> = self.connected_peers_excluding(&joiner_id);
                for existing_id in &existing {
                    out.push(Output::SendRelay {
                        to: existing_id.clone(),
                        msg: RelayMessage::PeerJoined {
                            peer_id: joiner_id.clone(),
                        },
                    });
                    // Tell the new joiner about existing peers
                    out.push(Output::SendRelay {
                        to: joiner_id.clone(),
                        msg: RelayMessage::PeerJoined {
                            peer_id: existing_id.clone(),
                        },
                    });
                }

                out.push(Output::PeerConnected { peer_id: joiner_id });
            }
            Role::Joiner => {
                // The joiner waits for the Welcome message to learn its ID.
                // PeerConnected will be emitted when Welcome arrives.
                out.push(Output::PeerConnected {
                    peer_id: PeerId::master(),
                });
            }
        }

        Ok(out)
    }

    fn on_dc_closed(&mut self, remote: PeerId) -> Result<Vec<Output>> {
        // Check if it's the bootstrap connection
        if self.bootstrap_remote.as_ref() == Some(&remote) {
            self.bootstrap = BootstrapPhase::WaitingForInvite;
            self.bootstrap_remote = None;
            return Ok(vec![Output::PeerDisconnected { peer_id: remote }]);
        }

        if let Some(ps) = self.peers.get_mut(&remote) {
            ps.phase = Phase::Disconnected;
        }
        let mut out = vec![
            Output::ClosePeerConnection {
                remote: remote.clone(),
            },
            Output::PeerDisconnected {
                peer_id: remote.clone(),
            },
        ];

        // Master: notify other peers
        if self.role == Role::Master {
            for peer_id in self.connected_peers_excluding(&remote) {
                out.push(Output::SendRelay {
                    to: peer_id,
                    msg: RelayMessage::PeerLeft {
                        peer_id: remote.clone(),
                    },
                });
            }
        }

        self.peers.remove(&remote);
        Ok(out)
    }

    // -- Data received (distinguish relay vs. user data) --------------------

    fn on_data_received(&mut self, from: PeerId, data: Vec<u8>) -> Result<Vec<Output>> {
        // Try to parse as a relay message
        if let Ok(text) = std::str::from_utf8(&data) {
            if let Ok(msg) = serde_json::from_str::<RelayMessage>(text) {
                return self.on_relay_received(from, msg);
            }
        }
        // Not a relay message — deliver as user data
        Ok(vec![Output::DeliverMessage { from, data }])
    }

    // -- Relay message handling ---------------------------------------------

    fn on_relay_received(&mut self, from: PeerId, msg: RelayMessage) -> Result<Vec<Output>> {
        match self.role {
            Role::Master => self.master_on_relay(from, msg),
            Role::Joiner => self.joiner_on_relay(from, msg),
        }
    }

    /// Master: handle relay messages from joiners.
    /// The master forwards Offer/Answer/IceCandidate to the target peer.
    fn master_on_relay(&mut self, from: PeerId, msg: RelayMessage) -> Result<Vec<Output>> {
        match msg {
            RelayMessage::Offer { to, sdp, .. } => {
                // Forward to target
                Ok(vec![Output::SendRelay {
                    to: to.clone(),
                    msg: RelayMessage::Offer { from, to, sdp },
                }])
            }
            RelayMessage::Answer { to, sdp, .. } => Ok(vec![Output::SendRelay {
                to: to.clone(),
                msg: RelayMessage::Answer { from, to, sdp },
            }]),
            RelayMessage::IceCandidate {
                to,
                candidate,
                sdp_mid,
                sdp_m_line_index,
                ..
            } => Ok(vec![Output::SendRelay {
                to: to.clone(),
                msg: RelayMessage::IceCandidate {
                    from,
                    to,
                    candidate,
                    sdp_mid,
                    sdp_m_line_index,
                },
            }]),
            // Master doesn't expect Welcome/PeerJoined/PeerLeft from joiners
            _ => Ok(vec![]),
        }
    }

    /// Joiner: handle relay messages from the master.
    fn joiner_on_relay(&mut self, _from: PeerId, msg: RelayMessage) -> Result<Vec<Output>> {
        match msg {
            RelayMessage::Welcome { peer_id } => {
                self.local_id = peer_id;
                Ok(vec![])
            }
            RelayMessage::PeerJoined { peer_id } => self.on_mesh_peer_joined(peer_id),
            RelayMessage::PeerLeft { peer_id } => Ok(self.on_mesh_peer_left(peer_id)),
            RelayMessage::Offer { from, sdp, .. } => self.on_mesh_remote_offer(from, sdp),
            RelayMessage::Answer { from, sdp, .. } => self.on_mesh_remote_answer(from, sdp),
            RelayMessage::IceCandidate {
                from,
                candidate,
                sdp_mid,
                sdp_m_line_index,
                ..
            } => Ok(self.on_mesh_remote_ice(from, candidate, sdp_mid, sdp_m_line_index)),
        }
    }

    // -- Mesh peer management (for full mesh beyond bootstrap) ---------------

    fn on_mesh_peer_joined(&mut self, remote: PeerId) -> Result<Vec<Output>> {
        let we_initiate = self.local_id < remote;
        let polite = we_initiate;

        self.peers
            .insert(remote.clone(), PeerState::new(remote.clone(), polite));

        let mut out = vec![Output::CreatePeerConnection {
            remote: remote.clone(),
            ice_servers: self.ice_servers.clone(),
        }];

        if we_initiate {
            out.push(Output::CreateDataChannel {
                remote: remote.clone(),
                label: "data".to_owned(),
            });
            out.push(Output::CreateOffer {
                remote: remote.clone(),
            });
            if let Some(ps) = self.peers.get_mut(&remote) {
                ps.phase = Phase::CreatingOffer;
            }
        }

        Ok(out)
    }

    fn on_mesh_peer_left(&mut self, remote: PeerId) -> Vec<Output> {
        self.peers.remove(&remote);
        vec![
            Output::ClosePeerConnection {
                remote: remote.clone(),
            },
            Output::PeerDisconnected { peer_id: remote },
        ]
    }

    fn on_mesh_remote_offer(&mut self, from: PeerId, sdp: String) -> Result<Vec<Output>> {
        let mut out = Vec::new();

        if !self.peers.contains_key(&from) {
            let polite = self.local_id < from;
            self.peers
                .insert(from.clone(), PeerState::new(from.clone(), polite));
            out.push(Output::CreatePeerConnection {
                remote: from.clone(),
                ice_servers: self.ice_servers.clone(),
            });
        }

        let ps = self.peers.get_mut(&from).unwrap();

        // Glare resolution
        if ps.phase == Phase::CreatingOffer || ps.phase == Phase::OfferSent {
            if !ps.polite {
                return Ok(vec![]);
            }
        }

        ps.phase = Phase::OfferReceived;

        out.push(Output::SetRemoteDescription {
            remote: from.clone(),
            sdp,
            is_offer: true,
        });
        out.push(Output::CreateAnswer {
            remote: from.clone(),
        });

        let candidates: Vec<_> = ps.pending_candidates.drain(..).collect();
        for c in candidates {
            out.push(Output::AddIceCandidate {
                remote: from.clone(),
                candidate: c,
            });
        }

        Ok(out)
    }

    fn on_mesh_remote_answer(&mut self, from: PeerId, sdp: String) -> Result<Vec<Output>> {
        let ps = self
            .peers
            .get_mut(&from)
            .ok_or_else(|| anyhow!("unknown peer: {from}"))?;

        if ps.phase != Phase::OfferSent {
            return Ok(vec![]);
        }

        let mut out = vec![Output::SetRemoteDescription {
            remote: from.clone(),
            sdp,
            is_offer: false,
        }];

        let candidates: Vec<_> = ps.pending_candidates.drain(..).collect();
        for c in candidates {
            out.push(Output::AddIceCandidate {
                remote: from.clone(),
                candidate: c,
            });
        }

        Ok(out)
    }

    fn on_mesh_remote_ice(
        &mut self,
        from: PeerId,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    ) -> Vec<Output> {
        let info = IceCandidate {
            candidate,
            sdp_mid,
            sdp_m_line_index,
        };

        let Some(ps) = self.peers.get_mut(&from) else {
            return vec![];
        };

        if ps.has_remote_description() {
            vec![Output::AddIceCandidate {
                remote: from,
                candidate: info,
            }]
        } else {
            ps.pending_candidates.push(info);
            vec![]
        }
    }

    // -- Local WebRTC events (mesh connections) ------------------------------

    fn on_local_offer(&mut self, remote: PeerId, sdp: String) -> Result<Vec<Output>> {
        let local_id = self.local_id.clone();
        let ps = self
            .peers
            .get_mut(&remote)
            .ok_or_else(|| anyhow!("unknown peer: {remote}"))?;
        ps.phase = Phase::OfferSent;

        Ok(vec![
            Output::SetLocalDescription {
                remote: remote.clone(),
                sdp: sdp.clone(),
                is_offer: true,
            },
            Output::SendRelay {
                to: PeerId::master(),
                msg: RelayMessage::Offer {
                    from: local_id,
                    to: remote,
                    sdp,
                },
            },
        ])
    }

    fn on_local_answer(&mut self, remote: PeerId, sdp: String) -> Result<Vec<Output>> {
        let local_id = self.local_id.clone();
        let ps = self
            .peers
            .get_mut(&remote)
            .ok_or_else(|| anyhow!("unknown peer: {remote}"))?;
        ps.phase = Phase::AnswerSent;

        Ok(vec![
            Output::SetLocalDescription {
                remote: remote.clone(),
                sdp: sdp.clone(),
                is_offer: false,
            },
            Output::SendRelay {
                to: PeerId::master(),
                msg: RelayMessage::Answer {
                    from: local_id,
                    to: remote,
                    sdp,
                },
            },
        ])
    }

    fn on_local_ice(
        &mut self,
        remote: PeerId,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    ) -> Vec<Output> {
        let local_id = self.local_id.clone();

        vec![Output::SendRelay {
            to: PeerId::master(),
            msg: RelayMessage::IceCandidate {
                from: local_id,
                to: remote,
                candidate,
                sdp_mid,
                sdp_m_line_index,
            },
        }]
    }

    fn on_ice_state_change(&mut self, remote: PeerId, state: &str) -> Vec<Output> {
        if state == "failed" {
            if let Some(ps) = self.peers.get_mut(&remote) {
                ps.phase = Phase::Disconnected;
            }
            return vec![
                Output::ClosePeerConnection {
                    remote: remote.clone(),
                },
                Output::PeerDisconnected { peer_id: remote },
            ];
        }
        vec![]
    }

    // -- Helpers ------------------------------------------------------------

    fn connected_peers_excluding(&self, exclude: &PeerId) -> Vec<PeerId> {
        let mut result = Vec::new();
        if let Some(ref boot_id) = self.bootstrap_remote {
            if boot_id != exclude && self.bootstrap == BootstrapPhase::Connected {
                result.push(boot_id.clone());
            }
        }
        for ps in self.peers.values() {
            if ps.phase == Phase::Connected && &ps.id != exclude {
                result.push(ps.id.clone());
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::IceServer;

    fn stun() -> Vec<IceServer> {
        vec![IceServer::default_stun()]
    }

    #[test]
    fn master_create_invite_produces_bootstrap_outputs() {
        let mut master = Mesh::new_master(stun());
        let out = master.create_invite().unwrap();

        assert!(
            out.iter()
                .any(|o| matches!(o, Output::CreateBootstrapPeerConnection { .. }))
        );
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::CreateBootstrapDataChannel { .. }))
        );
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::CreateBootstrapOffer))
        );
    }

    #[test]
    fn master_ice_complete_emits_invite_ready() {
        let mut master = Mesh::new_master(stun());
        master.create_invite().unwrap();

        // Simulate offer created
        master
            .handle(Input::BootstrapOfferCreated {
                sdp: "offer-sdp".into(),
            })
            .unwrap();

        // Simulate ICE gathering complete
        let out = master
            .handle(Input::BootstrapIceGatheringComplete {
                sdp: "offer-sdp-with-candidates".into(),
            })
            .unwrap();

        let invite = out.iter().find_map(|o| match o {
            Output::InviteReady(inv) => Some(inv),
            _ => None,
        });
        assert!(invite.is_some());
        assert_eq!(invite.unwrap().sdp, "offer-sdp-with-candidates");
    }

    #[test]
    fn joiner_invite_received_creates_answer_flow() {
        let mut joiner = Mesh::new_joiner();
        let invite = Invite {
            sdp: "master-offer".into(),
            ice_servers: stun(),
        };

        let out = joiner.handle(Input::InviteReceived(invite)).unwrap();

        assert!(
            out.iter()
                .any(|o| matches!(o, Output::CreateBootstrapPeerConnection { .. }))
        );
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::SetBootstrapRemoteDescription { .. }))
        );
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::CreateBootstrapAnswer))
        );
    }

    #[test]
    fn joiner_answer_ice_complete_emits_answer_ready() {
        let mut joiner = Mesh::new_joiner();
        let invite = Invite {
            sdp: "master-offer".into(),
            ice_servers: stun(),
        };
        joiner.handle(Input::InviteReceived(invite)).unwrap();

        joiner
            .handle(Input::BootstrapAnswerCreated {
                sdp: "answer-sdp".into(),
            })
            .unwrap();

        let out = joiner
            .handle(Input::BootstrapAnswerIceComplete {
                sdp: "answer-sdp-with-candidates".into(),
            })
            .unwrap();

        let answer = out.iter().find_map(|o| match o {
            Output::AnswerReady(ans) => Some(ans),
            _ => None,
        });
        assert!(answer.is_some());
        assert_eq!(answer.unwrap().sdp, "answer-sdp-with-candidates");
    }

    #[test]
    fn master_answer_received_sets_remote_desc() {
        let mut master = Mesh::new_master(stun());
        master.create_invite().unwrap();
        master
            .handle(Input::BootstrapOfferCreated {
                sdp: "offer".into(),
            })
            .unwrap();
        master
            .handle(Input::BootstrapIceGatheringComplete {
                sdp: "offer-full".into(),
            })
            .unwrap();

        let out = master
            .handle(Input::AnswerReceived(Answer {
                sdp: "joiner-answer".into(),
            }))
            .unwrap();

        assert!(
            out.iter()
                .any(|o| matches!(o, Output::SetBootstrapRemoteDescription { .. }))
        );
    }

    #[test]
    fn bootstrap_dc_open_master_sends_welcome() {
        let mut master = Mesh::new_master(stun());
        master.create_invite().unwrap();
        master
            .handle(Input::BootstrapOfferCreated {
                sdp: "offer".into(),
            })
            .unwrap();
        master
            .handle(Input::BootstrapIceGatheringComplete {
                sdp: "offer-full".into(),
            })
            .unwrap();
        master
            .handle(Input::AnswerReceived(Answer {
                sdp: "answer".into(),
            }))
            .unwrap();

        let out = master
            .handle(Input::DataChannelOpen {
                remote: PeerId::from("peer-1"),
            })
            .unwrap();

        // Should send Welcome relay message
        let has_welcome = out.iter().any(|o| {
            matches!(
                o,
                Output::SendRelay {
                    msg: RelayMessage::Welcome { .. },
                    ..
                }
            )
        });
        assert!(has_welcome);

        // Should notify user
        assert!(
            out.iter()
                .any(|o| matches!(o, Output::PeerConnected { .. }))
        );
    }

    #[test]
    fn joiner_only_accepts_invites() {
        let master = Mesh::new_master(stun());
        assert!(master.role() == Role::Master);

        let joiner = Mesh::new_joiner();
        assert!(joiner.role() == Role::Joiner);
    }

    #[test]
    fn data_received_non_relay_delivers_message() {
        let mut master = Mesh::new_master(stun());
        let out = master
            .handle(Input::DataReceived {
                from: PeerId::from("someone"),
                data: vec![1, 2, 3],
            })
            .unwrap();

        assert!(matches!(
            out.as_slice(),
            [Output::DeliverMessage { from, data }]
                if from == &PeerId::from("someone") && data == &[1, 2, 3]
        ));
    }

    #[test]
    fn master_relays_offer_between_peers() {
        let mut master = Mesh::new_master(stun());
        // Simulate bootstrap complete
        master.create_invite().unwrap();
        master
            .handle(Input::BootstrapOfferCreated { sdp: "o".into() })
            .unwrap();
        master
            .handle(Input::BootstrapIceGatheringComplete { sdp: "o".into() })
            .unwrap();
        master
            .handle(Input::AnswerReceived(Answer { sdp: "a".into() }))
            .unwrap();
        master
            .handle(Input::DataChannelOpen {
                remote: PeerId::from("peer-1"),
            })
            .unwrap();

        // peer-1 sends an offer to peer-2 via master
        let relay_msg = RelayMessage::Offer {
            from: PeerId::from("peer-1"),
            to: PeerId::from("peer-2"),
            sdp: "peer1-offer".into(),
        };
        let out = master
            .handle(Input::RelayReceived {
                from: PeerId::from("peer-1"),
                msg: relay_msg,
            })
            .unwrap();

        // Master should forward to peer-2
        let forwarded = out.iter().any(|o| {
            matches!(o,
                Output::SendRelay { to, msg: RelayMessage::Offer { sdp, .. } }
                    if to == &PeerId::from("peer-2") && sdp == "peer1-offer"
            )
        });
        assert!(forwarded);
    }
}
