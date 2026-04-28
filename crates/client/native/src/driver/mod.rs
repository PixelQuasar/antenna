use crate::{ConnectionManager, DataChannelManager, Storage};
use antenna_client_shared::{
    EXECUTE_FUEL, EventType, IceServerConfig, IdentityStorage, RtcCallbacks,
};
use antenna_protocol::{
    HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, MsgPayload, Output, PeerID, Scheduled,
    SignalingPayload, UserMsgPayload,
};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;

/// Native implementation of driver
pub struct Driver<Msg: UserMsgPayload + Send + Sync + 'static> {
    fsm: MeshNodeFSM,
    connections: HashMap<PeerID, Arc<ConnectionManager>>,
    pending: VecDeque<Arc<ConnectionManager>>,
    ice_servers: Vec<IceServerConfig>,
    callbacks: Arc<Mutex<RtcCallbacks<Msg>>>,
    storage: Storage,
}

impl<Msg: UserMsgPayload + Send + Sync + 'static> Driver<Msg> {
    pub fn new(
        ice_servers: Vec<IceServerConfig>,
        callbacks: Arc<Mutex<RtcCallbacks<Msg>>>,
        storage: Storage,
    ) -> Self {
        let identity = storage.load_identity();
        let fsm = match identity {
            Some(id) => MeshNodeFSM::with_identity(id),
            None => MeshNodeFSM::new(),
        };
        Self {
            fsm,
            connections: HashMap::new(),
            pending: VecDeque::new(),
            ice_servers,
            callbacks,
            storage,
        }
    }

    pub fn id(&self) -> &PeerID {
        self.fsm.id()
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.fsm.is_connected(peer)
    }

    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.fsm.connected_peers()
    }

    /// Helper: emit a single event without holding the driver lock during dispatch
    async fn emit(driver: &Arc<Mutex<Self>>, event: EventType<Msg>) -> Result<()> {
        let cbs = driver.lock().await.callbacks.clone();
        cbs.lock().await.emit(event)
    }

    /// Spawn an async task that runs an FSM input through the execute loop
    pub(crate) fn dispatch_input(
        driver: Arc<Mutex<Self>>,
        input: Input<Msg>,
        context: &'static str,
    ) {
        tokio::spawn(async move {
            if let Err(e) = Driver::execute(driver, input).await {
                eprintln!("{context}: {e:#}");
            }
        });
    }

    /// Execute fsm input and handle its output as side effect on the platform layer.
    /// Returns the outputs that aren't consumed as side effects
    pub async fn execute(driver: Arc<Mutex<Self>>, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        let outputs = {
            let mut d = driver.lock().await;
            d.fsm.process(input)?
        };
        let mut queue = VecDeque::from(outputs);
        let mut returned: Vec<Output<Msg>> = Vec::new();
        let mut fuel = EXECUTE_FUEL;

        while let Some(output) = queue.pop_back()
            && fuel > 0
        {
            fuel -= 1;
            let new_outputs: Vec<Output<Msg>> = match output {
                Output::InitOpenOffer => {
                    Self::execute_handshake(driver.clone(), None, HandshakeOutput::InitSDPOffer)
                        .await?
                }
                output @ (Output::OfferReady(_) | Output::AnswerReady(_)) => {
                    returned.push(output);
                    vec![]
                }
                Output::Handshake { peer, event } => {
                    Self::execute_handshake(driver.clone(), Some(peer), event).await?
                }
                Output::SendMessage { peer_to, data } => {
                    Self::execute_send(&driver, &peer_to, &data).await?
                }
                Output::ReceiveMessage { peer_from, data } => {
                    if let MsgPayload::User(data) = data {
                        Self::emit(&driver, EventType::UserMessage(peer_from, data)).await?;
                    }
                    vec![]
                }
                Output::PeerConnected { peer } => {
                    {
                        let d = driver.lock().await;
                        if let Err(err) = d.storage.save_identity(d.fsm.identity()) {
                            eprintln!("Error during identity save: {err:?}");
                        }
                    }
                    Self::emit(&driver, EventType::PeerConnected(peer)).await?;
                    vec![]
                }
                Output::PeerDisconnected { peer } => {
                    let conn = {
                        let mut d = driver.lock().await;
                        d.connections.remove(&peer)
                    };
                    if let Some(conn) = conn {
                        conn.close().await;
                    }
                    Self::emit(&driver, EventType::PeerDisconnected(peer)).await?;
                    vec![]
                }
                Output::PeerLost { peer } => {
                    Self::emit(&driver, EventType::PeerDropped(peer)).await?;
                    vec![]
                }
                Output::Available => {
                    Self::emit(&driver, EventType::Available).await?;
                    vec![]
                }
                Output::Unavailable => {
                    Self::emit(&driver, EventType::Unavailable).await?;
                    vec![]
                }
                Output::Disconnecting => {
                    let (conns, pending) = {
                        let mut d = driver.lock().await;
                        let conns: Vec<_> = d.connections.drain().map(|(_, v)| v).collect();
                        let pending: Vec<_> = d.pending.drain(..).collect();
                        (conns, pending)
                    };
                    // Brief grace before closing PCs: give the just-sent `Disconnect`
                    // messages a chance to flush through SCTP. webrtc-rs's PC.close()
                    // can be abrupt enough that buffered data is dropped, leaving the
                    // remote without a graceful-disconnect signal.
                    for conn in conns.into_iter().chain(pending) {
                        conn.close().await;
                    }
                    Self::emit(&driver, EventType::Disconnected).await?;
                    vec![]
                }
                Output::ScheduleTimer { kind, after_ms } => {
                    Self::execute_timer(driver.clone(), kind, after_ms);
                    vec![]
                }
            };
            queue.extend(new_outputs);
        }
        Ok(returned)
    }

    async fn execute_send(
        driver: &Arc<Mutex<Self>>,
        peer: &PeerID,
        data: &MsgPayload<Msg>,
    ) -> Result<Vec<Output<Msg>>> {
        let conn = {
            let d = driver.lock().await;
            d.connections.get(peer).cloned().context("Peer not found")?
        };
        let dc_guard = conn.dc().lock().await;
        if let Some(dc) = dc_guard.as_ref() {
            dc.send_data(data).await?;
        }
        Ok(vec![])
    }

    async fn execute_handshake(
        driver: Arc<Mutex<Self>>,
        peer: Option<PeerID>,
        output: HandshakeOutput,
    ) -> Result<Vec<Output<Msg>>> {
        match output {
            HandshakeOutput::InitSDPOffer => Self::execute_init_offer(driver, peer).await,
            HandshakeOutput::RequestSDPAnswer(offer) => {
                Self::execute_init_answer(driver, peer, offer).await
            }
            HandshakeOutput::AcceptSDPAnswer(answer) => {
                Self::execute_accept_answer(driver, peer, answer).await
            }
            HandshakeOutput::Close => Self::execute_close(driver, peer).await,
            HandshakeOutput::Connected => Self::execute_connected(driver).await,
        }
    }

    /// Schedule a one-shot timer that feeds `Input::TimerFired { kind }` back into the FSM
    fn execute_timer(driver: Arc<Mutex<Self>>, kind: Scheduled, after_ms: u64) {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(after_ms)).await;
            if let Err(e) = Driver::execute(driver, Input::TimerFired { kind }).await {
                eprintln!("TimerFired: {e:#}");
            }
        });
    }

    /// Observe ICE state and fire `HandshakeInput::ConnectionDropped` on `Failed`
    fn attach_ice_state_observer(
        peer_id: PeerID,
        driver: Arc<Mutex<Self>>,
        conn: &ConnectionManager,
    ) {
        conn.setup_on_ice_state_change(move |state| {
            if state == RTCIceConnectionState::Failed {
                Self::dispatch_input(
                    driver.clone(),
                    Input::Handshake {
                        from: peer_id.clone(),
                        event: HandshakeInput::ConnectionDropped,
                    },
                    "ICE-driven ConnectionDropped",
                );
            }
        });
    }

    async fn execute_init_offer(
        driver: Arc<Mutex<Self>>,
        peer: Option<PeerID>,
    ) -> Result<Vec<Output<Msg>>> {
        let ice_servers = driver.lock().await.ice_servers.clone();
        let conn = Arc::new(ConnectionManager::new_host(&ice_servers).await?);

        let sdp = conn.create_offer().await?;

        if let Some(peer_id) = &peer {
            Self::attach_ice_state_observer(peer_id.clone(), driver.clone(), &conn);
        }

        let fsm_input = match &peer {
            None => Input::<Msg>::OpenOfferCreated(sdp),
            Some(peer_id) => Input::<Msg>::Handshake {
                from: peer_id.clone(),
                event: HandshakeInput::OfferCreated(sdp),
            },
        };
        let outputs = {
            let mut d = driver.lock().await;
            d.fsm.process(fsm_input)?
        };

        {
            let mut d = driver.lock().await;
            match peer {
                None => d.pending.push_back(conn),
                Some(peer_id) => {
                    d.connections.insert(peer_id, conn);
                }
            }
        }

        Ok(outputs)
    }

    async fn execute_init_answer(
        driver: Arc<Mutex<Self>>,
        peer: Option<PeerID>,
        offer: SignalingPayload,
    ) -> Result<Vec<Output<Msg>>> {
        let peer_id = peer.context("joiner always has a known peer ID")?;
        let ice_servers = driver.lock().await.ice_servers.clone();
        let conn = Arc::new(ConnectionManager::new_joiner(&ice_servers).await?);

        Self::setup_joiner_data_channel(driver.clone(), &peer_id, conn.clone());
        Self::attach_ice_state_observer(peer_id.clone(), driver.clone(), &conn);

        let sdp = conn.create_answer(&offer.get_sdp_verified(&peer_id)?).await?;

        let outputs = {
            let mut d = driver.lock().await;
            d.fsm.process(Input::<Msg>::Handshake {
                from: peer_id.clone(),
                event: HandshakeInput::AnswerCreated(sdp),
            })?
        };

        driver.lock().await.connections.insert(peer_id, conn);

        Ok(outputs)
    }

    async fn execute_accept_answer(
        driver: Arc<Mutex<Self>>,
        peer: Option<PeerID>,
        answer: SignalingPayload,
    ) -> Result<Vec<Output<Msg>>> {
        let peer_id = peer.context("accept answer called without peer ID")?;

        {
            let mut d = driver.lock().await;
            if !d.connections.contains_key(&peer_id)
                && let Some(pending) = d.pending.pop_front()
            {
                d.connections.insert(peer_id.clone(), pending);
            }
        }

        let conn = {
            let d = driver.lock().await;
            d.connections
                .get(&peer_id)
                .cloned()
                .context("Connection not found for peer")?
        };

        Self::attach_ice_state_observer(peer_id.clone(), driver.clone(), &conn);

        {
            let dc_guard = conn.dc().lock().await;
            let dc = dc_guard.as_ref().context("DataChannel not initialized")?;
            Self::attach_data_channel_callbacks(peer_id, driver.clone(), dc);
        }

        conn.accept_answer(&answer.get_sdp_verified(&peer_id)?).await?;

        Ok(vec![])
    }

    async fn execute_close(
        driver: Arc<Mutex<Self>>,
        peer: Option<PeerID>,
    ) -> Result<Vec<Output<Msg>>> {
        let to_close: Vec<Arc<ConnectionManager>> = {
            let mut d = driver.lock().await;
            match &peer {
                None => d.pending.drain(..).collect(),
                Some(peer_id) => d.connections.remove(peer_id).into_iter().collect(),
            }
        };
        for conn in to_close {
            conn.close().await;
        }
        Self::emit(&driver, EventType::Disconnected).await?;
        Ok(vec![])
    }

    async fn execute_connected(driver: Arc<Mutex<Self>>) -> Result<Vec<Output<Msg>>> {
        Self::emit(&driver, EventType::Connected).await?;
        Ok(vec![])
    }

    /// Joiner-side: setup datachannel manager
    fn setup_joiner_data_channel(
        driver: Arc<Mutex<Self>>,
        peer_id: &PeerID,
        conn: Arc<ConnectionManager>,
    ) {
        let peer_id = peer_id.clone();
        let conn_for_cb = conn.clone();
        conn.setup_on_data_channel(move |dc: Arc<RTCDataChannel>| {
            let dc_manager = DataChannelManager::from_existing(dc);
            Self::attach_data_channel_callbacks(peer_id.clone(), driver.clone(), &dc_manager);
            let conn_for_cb = conn_for_cb.clone();
            tokio::spawn(async move {
                conn_for_cb.set_dc(dc_manager).await;
            });
        });
    }

    fn attach_data_channel_callbacks(
        peer_id: PeerID,
        driver: Arc<Mutex<Self>>,
        dc_manager: &DataChannelManager,
    ) {
        {
            let peer_id = peer_id.clone();
            let driver = driver.clone();
            dc_manager.setup_on_open(move || {
                Self::dispatch_input(
                    driver.clone(),
                    Input::Handshake {
                        from: peer_id.clone(),
                        event: HandshakeInput::DataChannelOpen,
                    },
                    "DataChannelOpen",
                );
            });
        }

        {
            let peer_id = peer_id.clone();
            let driver = driver.clone();
            dc_manager.setup_on_message(move |bytes| {
                let data: MsgPayload<Msg> = match serde_json::from_slice(&bytes) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("Failed to deserialize incoming message: {err:#}");
                        return;
                    }
                };
                Self::dispatch_input(
                    driver.clone(),
                    Input::MessageReceived {
                        peer_from: peer_id.clone(),
                        data,
                    },
                    "MessageReceived",
                );
            });
        }

        {
            let driver = driver.clone();
            dc_manager.setup_on_close(move || {
                Self::dispatch_input(
                    driver.clone(),
                    Input::Handshake {
                        from: peer_id.clone(),
                        event: HandshakeInput::ConnectionDropped,
                    },
                    "ConnectionDropped",
                );
            });
        }
    }
}
