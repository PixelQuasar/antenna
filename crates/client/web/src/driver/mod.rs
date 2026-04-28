use crate::{ConnectionManager, DataChannelManager, Storage};
use antenna_client_shared::{
    EXECUTE_FUEL, EventType, IceServerConfig, IdentityStorage, RtcCallbacks,
};
use antenna_protocol::{
    HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeStrategy, Input, MeshNodeFSM,
    MsgPayload, Output, PeerID, Scheduled, SignalingPayload, UserMsgPayload,
};
use anyhow::{Context, Result, anyhow};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub struct Driver<Msg: UserMsgPayload + 'static> {
    fsm: MeshNodeFSM,
    connections: HashMap<PeerID, Rc<ConnectionManager>>,
    pending: VecDeque<Rc<ConnectionManager>>,
    ice_servers: Vec<IceServerConfig>,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    storage: Storage,
}

impl<Msg: UserMsgPayload + 'static> Driver<Msg> {
    pub fn new(
        ice_servers: Vec<IceServerConfig>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
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

    /// Peer id getter
    pub fn id(&self) -> &PeerID {
        self.fsm.id()
    }

    /// Check is remote peer connected
    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.fsm.is_connected(peer)
    }

    /// Set of connected peers
    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.fsm.connected_peers()
    }

    /// Push an outbound `MsgPayload` over the data channel to a known peer.
    fn dispatch_outbound(&self, peer: &PeerID, data: &MsgPayload<Msg>) -> Result<Vec<Output<Msg>>> {
        let conn = self.connections.get(peer).context("Peer not found")?;
        if let Some(dc) = conn.dc().borrow().as_ref() {
            dc.send_data(data)?;
        }
        Ok(vec![])
    }

    /// Emit a single event
    fn emit(driver: &Rc<RefCell<Self>>, event: EventType<Msg>) -> Result<()> {
        driver.borrow().callbacks.borrow().emit(event)
    }

    /// Bootstrap host: create an open SDP offer and return it base64-encoded.
    pub async fn start(driver: Rc<RefCell<Self>>) -> Result<String> {
        let outputs = Self::execute(driver, Input::InitOpenOffer).await?;
        outputs
            .into_iter()
            .find_map(|o| match o {
                Output::OfferReady(payload) => Some(payload),
                _ => None,
            })
            .context("Offer not found on starting")?
            .to_base64()
    }

    /// Bootstrap joiner: accept the host's base64 offer, return our base64 answer.
    pub async fn receive_offer(driver: Rc<RefCell<Self>>, offer: &str) -> Result<String> {
        let offer = SignalingPayload::from_base64(offer)?;
        let peer_id = offer.peer_id();
        Self::execute(
            driver.clone(),
            Input::InitHandshake {
                with: peer_id.clone(),
                mode: HandshakeMode::Bootstrap,
                strategy: HandshakeStrategy::Joiner,
            },
        )
        .await?;
        let outputs = Self::execute(
            driver,
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Offer(offer),
            },
        )
        .await?;
        outputs
            .into_iter()
            .find_map(|o| match o {
                Output::AnswerReady(payload) => Some(payload),
                _ => None,
            })
            .context("Answer not found on receiving offer")?
            .to_base64()
    }

    /// Bootstrap host: accept the joiner's base64 answer to complete the handshake.
    pub async fn receive_answer(driver: Rc<RefCell<Self>>, answer: &str) -> Result<()> {
        let answer = SignalingPayload::from_base64(answer)?;
        let peer_id = answer.peer_id();
        Self::execute(
            driver,
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::Answer(answer),
            },
        )
        .await?;
        Ok(())
    }

    /// Fire-and-forget user message to a single peer.
    pub fn send(driver: Rc<RefCell<Self>>, peer_to: PeerID, data: Msg) {
        Self::dispatch_input(
            driver,
            Input::Send {
                peer_to,
                data: MsgPayload::User(data),
            },
            "Peer::send",
        );
    }

    /// Fire-and-forget user message to every connected peer.
    pub fn broadcast(driver: Rc<RefCell<Self>>, data: Msg) {
        Self::dispatch_input(
            driver,
            Input::Broadcast {
                data: MsgPayload::User(data),
            },
            "Peer::broadcast",
        );
    }

    /// Initiate graceful departure from the mesh.
    pub fn leave(driver: Rc<RefCell<Self>>) {
        Self::dispatch_input(driver, Input::Leave, "Peer::leave");
    }

    /// Spawn an async task that runs an FSM input
    fn dispatch_input(driver: Rc<RefCell<Self>>, input: Input<Msg>, context: &'static str) {
        spawn_local(async move {
            if let Err(e) = Driver::execute(driver, input).await {
                web_sys::console::error_1(&JsValue::from_str(&format!("{context}: {e:#}")));
            }
        });
    }

    /// Execute fsm input and handle its output as side effect on platform layer entities.
    /// Returns the outputs that aren't consumed as side effects.
    async fn execute(driver: Rc<RefCell<Self>>, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        let outputs = driver.borrow_mut().fsm.process(input)?;
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
                    driver.borrow().dispatch_outbound(&peer_to, &data)?
                }
                Output::ReceiveMessage { peer_from, data } => {
                    if let MsgPayload::User(data) = data {
                        Self::emit(&driver, EventType::UserMessage(peer_from, data))?;
                    }
                    vec![]
                }
                Output::PeerConnected { peer } => {
                    {
                        let d = driver.borrow();
                        if let Err(err) = d.storage.save_identity(d.fsm.identity()) {
                            web_sys::console::log_1(&JsValue::from_str(&format!(
                                "Error during identity save: {err:?}"
                            )));
                        }
                    }
                    Self::emit(&driver, EventType::PeerConnected(peer))?;
                    vec![]
                }
                Output::PeerDisconnected { peer } => {
                    if let Some(conn) = driver.borrow_mut().connections.remove(&peer) {
                        conn.close();
                    }
                    Self::emit(&driver, EventType::PeerDisconnected(peer))?;
                    vec![]
                }
                Output::PeerLost { peer } => {
                    Self::emit(&driver, EventType::PeerDropped(peer))?;
                    vec![]
                }
                Output::Connected => {
                    Self::emit(&driver, EventType::Connected)?;
                    vec![]
                }
                Output::Available => {
                    Self::emit(&driver, EventType::Available)?;
                    vec![]
                }
                Output::Unavailable => {
                    Self::emit(&driver, EventType::Unavailable)?;
                    vec![]
                }
                Output::Disconnecting => {
                    let (conns, pending) = {
                        let mut d = driver.borrow_mut();
                        let conns: Vec<_> = d.connections.drain().map(|(_, v)| v).collect();
                        let pending: Vec<_> = d.pending.drain(..).collect();
                        (conns, pending)
                    };
                    for conn in conns.into_iter().chain(pending) {
                        conn.close();
                    }
                    Self::emit(&driver, EventType::Disconnected)?;
                    vec![]
                }
                Output::ScheduleTimer { kind, after_ms } => {
                    Self::execute_timer(driver.clone(), kind, after_ms)?;
                    vec![]
                }
            };
            queue.extend(new_outputs);
        }
        Ok(returned)
    }

    /// Execute handshake input
    async fn execute_handshake(
        driver: Rc<RefCell<Self>>,
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
            HandshakeOutput::Connected => Ok(vec![]),
        }
    }

    /// Execute timer input
    fn execute_timer(driver: Rc<RefCell<Self>>, kind: Scheduled, after_ms: u64) -> Result<()> {
        let window = web_sys::window().context("no global window")?;

        let cb = Closure::once_into_js(move || {
            Self::dispatch_input(driver, Input::TimerFired { kind }, "TimerFired");
        });
        let timeout = i32::try_from(after_ms).unwrap_or(i32::MAX);
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), timeout)
            .map_err(|e| anyhow!("set_timeout failed: {:?}", e))?;
        Ok(())
    }

    /// Observe ICE state and fire `HandshakeInput::ConnectionDropped` on `Failed`
    fn attach_ice_state_observer(
        peer_id: PeerID,
        driver: Rc<RefCell<Self>>,
        conn: &ConnectionManager,
    ) {
        conn.setup_on_ice_state_change(move |state| {
            if state == web_sys::RtcIceConnectionState::Failed {
                Self::execute_connection_drop(driver.clone(), peer_id.clone());
            }
        });
    }

    /// Execute connection drop (abruptly close) event
    fn execute_connection_drop(driver: Rc<RefCell<Self>>, peer_id: PeerID) {
        Self::dispatch_input(
            driver,
            Input::Handshake {
                from: peer_id,
                event: HandshakeInput::ConnectionDropped,
            },
            "ICE-driven ConnectionDropped",
        );
    }

    /// Execute init offer flow
    async fn execute_init_offer(
        driver: Rc<RefCell<Self>>,
        peer: Option<PeerID>,
    ) -> Result<Vec<Output<Msg>>> {
        let ice_servers = driver.borrow().ice_servers.clone();
        let conn = Rc::new(ConnectionManager::new_host(&ice_servers)?);

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
        let outputs = driver.borrow_mut().fsm.process(fsm_input)?;

        match peer {
            None => driver.borrow_mut().pending.push_back(conn),
            Some(peer_id) => {
                driver.borrow_mut().connections.insert(peer_id, conn);
            }
        }

        Ok(outputs)
    }

    /// Execute init asnwer flow
    async fn execute_init_answer(
        driver: Rc<RefCell<Self>>,
        peer: Option<PeerID>,
        offer: SignalingPayload,
    ) -> Result<Vec<Output<Msg>>> {
        let peer_id = peer.context("joiner always has a known peer ID")?;
        let ice_servers = driver.borrow().ice_servers.clone();
        let conn = Rc::new(ConnectionManager::new_joiner(&ice_servers)?);

        Self::setup_joiner_data_channel(driver.clone(), &peer_id, conn.clone())?;
        Self::attach_ice_state_observer(peer_id.clone(), driver.clone(), &conn);

        let sdp = conn
            .create_answer(&offer.get_sdp_verified(&peer_id)?)
            .await?;

        let outputs = driver.borrow_mut().fsm.process(Input::<Msg>::Handshake {
            from: peer_id.clone(),
            event: HandshakeInput::AnswerCreated(sdp),
        })?;

        driver.borrow_mut().connections.insert(peer_id, conn);

        Ok(outputs)
    }

    /// Execute accept answer flow
    async fn execute_accept_answer(
        driver: Rc<RefCell<Self>>,
        peer: Option<PeerID>,
        answer: SignalingPayload,
    ) -> Result<Vec<Output<Msg>>> {
        let peer_id = peer.context("accept answer called without peer ID")?;

        {
            let mut driver = driver.borrow_mut();
            if !driver.connections.contains_key(&peer_id)
                && let Some(pending) = driver.pending.pop_front()
            {
                driver.connections.insert(peer_id.clone(), pending);
            }
        }

        let conn = driver
            .borrow()
            .connections
            .get(&peer_id)
            .cloned()
            .context("Connection not found for peer")?;

        Self::attach_ice_state_observer(peer_id.clone(), driver.clone(), &conn);

        {
            let mut dc_guard = conn.dc().borrow_mut();
            let dc = dc_guard.as_mut().context("DataChannel not initialized")?;
            Self::attach_data_channel_callbacks(peer_id, driver, dc)?;
        }

        conn.accept_answer(&answer.get_sdp_verified(&peer_id)?)
            .await?;

        Ok(vec![])
    }

    /// Execute graceful close
    async fn execute_close(
        driver: Rc<RefCell<Self>>,
        peer: Option<PeerID>,
    ) -> Result<Vec<Output<Msg>>> {
        {
            let mut driver = driver.borrow_mut();
            match &peer {
                None => {
                    for conn in driver.pending.drain(..) {
                        conn.close();
                    }
                }
                Some(peer_id) => {
                    if let Some(conn) = driver.connections.remove(peer_id) {
                        conn.close();
                    }
                }
            }
        }
        driver
            .borrow()
            .callbacks
            .borrow()
            .emit(EventType::Disconnected)?;
        Ok(vec![])
    }

    /// Joiner-side: setup datachannel manager
    fn setup_joiner_data_channel(
        driver: Rc<RefCell<Self>>,
        peer_id: &PeerID,
        conn: Rc<ConnectionManager>,
    ) -> Result<()> {
        let peer_id = peer_id.clone();
        let conn_for_cb = conn.clone();
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let event: web_sys::RtcDataChannelEvent = evt.unchecked_into();
            let channel = event.channel();
            let mut dc_manager = DataChannelManager::from_existing(channel);

            let result = Self::attach_data_channel_callbacks(
                peer_id.clone(),
                driver.clone(),
                &mut dc_manager,
            );
            if let Err(e) = result {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Error while attaching data channel callbacks: {:?}",
                    e
                )));
            }
            conn_for_cb.set_dc(dc_manager);
        }));
        conn.rtc_peer_connection()
            .set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
        conn.store_ondatachannel_closure(cb);
        Ok(())
    }

    /// Setup datachannel callbacks
    fn attach_data_channel_callbacks(
        peer_id: PeerID,
        driver: Rc<RefCell<Self>>,
        dc_manager: &mut DataChannelManager,
    ) -> Result<()> {
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
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to deserialize incoming message: {err:#}"
                        )));
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
            let peer_id = peer_id.clone();
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

        Ok(())
    }
}
