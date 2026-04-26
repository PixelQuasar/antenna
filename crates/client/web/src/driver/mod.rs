use antenna_client_shared::{ICE_DISCONNECTED_GRACE_MS, IdentityStorage};

use crate::{ConnectionManager, DataChannelManager, RtcCallbacks, Storage};
use antenna_client_shared::{EXECUTE_FUEL, EventType, IceServerConfig};
use antenna_protocol::{
    HandshakeInput, HandshakeOutput, Input, MeshMetadata, MeshNodeFSM, MsgPayload, Output, PeerID,
    Scheduled, SignalingPayload, UserMsgPayload,
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
}

impl<Msg: UserMsgPayload + 'static> Driver<Msg> {
    pub fn new(
        ice_servers: Vec<IceServerConfig>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    ) -> Self {
        let identity = Storage::load_identity();
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
        }
    }

    pub fn id(&self) -> &PeerID {
        self.fsm.id()
    }

    pub fn metadata(&self) -> MeshMetadata {
        self.fsm.metadata().clone()
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.fsm.is_connected(peer)
    }

    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.fsm.connected_peers()
    }

    fn send(&self, peer: &PeerID, data: &MsgPayload<Msg>) -> Result<Vec<Output<Msg>>> {
        let conn = self.connections.get(peer).context("Peer not found")?;
        if let Some(dc) = conn.dc().borrow().as_ref() {
            dc.send_data(data)?;
        }
        Ok(vec![])
    }

    pub async fn execute(driver: Rc<RefCell<Self>>, input: Input<Msg>) -> Result<()> {
        let outputs = driver.borrow_mut().fsm.process(input)?;
        let mut queue = VecDeque::from(outputs);
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
                Output::Handshake { peer, event } => {
                    Self::execute_handshake(driver.clone(), Some(peer), event).await?
                }
                Output::SendMessage { peer_to, data } => driver.borrow().send(&peer_to, &data)?,
                Output::ReceiveMessage { peer_from, data } => {
                    if let MsgPayload::User(data) = data {
                        driver
                            .borrow()
                            .callbacks
                            .borrow()
                            .emit(EventType::UserMessage(peer_from, data))?;
                    }
                    vec![]
                }
                Output::PeerConnected { peer } => {
                    let driver = driver.borrow();
                    if let Err(err) = Storage::save_identity(driver.fsm.identity()) {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Error during identity save: {:?}",
                            err
                        )));
                    }
                    driver
                        .callbacks
                        .borrow()
                        .emit(EventType::PeerConnected(peer))?;
                    vec![]
                }
                Output::PeerDisconnected { peer } => {
                    if let Some(conn) = driver.borrow_mut().connections.remove(&peer) {
                        conn.close();
                    }
                    driver
                        .borrow()
                        .callbacks
                        .borrow()
                        .emit(EventType::PeerDisconnected(peer))?;
                    vec![]
                }
                Output::PeerLost { peer } => {
                    driver
                        .borrow()
                        .callbacks
                        .borrow()
                        .emit(EventType::PeerDropped(peer))?;
                    vec![]
                }
                Output::Available => {
                    driver
                        .borrow()
                        .callbacks
                        .borrow()
                        .emit(EventType::Available)?;
                    vec![]
                }
                Output::Unavailable => {
                    driver
                        .borrow()
                        .callbacks
                        .borrow()
                        .emit(EventType::Unavailable)?;
                    vec![]
                }
                Output::Disconnecting => {
                    let conns: Vec<_> = driver
                        .borrow_mut()
                        .connections
                        .drain()
                        .map(|(_, v)| v)
                        .collect();
                    let pending: Vec<_> = driver.borrow_mut().pending.drain(..).collect();
                    for conn in conns.into_iter().chain(pending) {
                        conn.close();
                    }
                    driver
                        .borrow()
                        .callbacks
                        .borrow()
                        .emit(EventType::Disconnected)?;
                    vec![]
                }
                Output::ScheduleTimer { kind, after_ms } => {
                    Self::schedule_timer(driver.clone(), kind, after_ms)?;
                    vec![]
                }
            };
            queue.extend(new_outputs);
        }
        Ok(())
    }

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
            HandshakeOutput::Connected => driver.borrow_mut().execute_connected(),
        }
    }

    fn schedule_timer(driver: Rc<RefCell<Self>>, kind: Scheduled, after_ms: u64) -> Result<()> {
        let window = web_sys::window().context("no global window")?;

        let cb = Closure::once_into_js(move || {
            spawn_local(async move {
                if let Err(e) = Driver::execute(driver, Input::TimerFired { kind }).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "TimerFired execution failed: {e:#}"
                    )));
                }
            });
        });

        let timeout = i32::try_from(after_ms).unwrap_or(i32::MAX);
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), timeout)
            .map_err(|e| anyhow!("set_timeout failed: {:?}", e))?;

        Ok(())
    }

    fn attach_ice_state_observer(
        peer_id: PeerID,
        driver: Rc<RefCell<Self>>,
        conn: &ConnectionManager,
    ) {
        let pc = conn.rtc_peer_connection().clone();

        conn.setup_on_ice_state_change(move |state| match state {
            web_sys::RtcIceConnectionState::Failed => {
                Self::execute_connection_dropped(driver.clone(), peer_id.clone());
            }
            web_sys::RtcIceConnectionState::Disconnected => {
                let driver = driver.clone();
                let peer_id = peer_id.clone();
                let pc = pc.clone();

                let cb = Closure::once_into_js(move || {
                    let now = pc.ice_connection_state();
                    if now == web_sys::RtcIceConnectionState::Disconnected
                        || now == web_sys::RtcIceConnectionState::Failed
                    {
                        Self::execute_connection_dropped(driver, peer_id);
                    }
                });
                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.unchecked_ref(),
                        ICE_DISCONNECTED_GRACE_MS as i32,
                    );
                }
            }
            _ => {}
        });
    }

    fn execute_connection_dropped(driver: Rc<RefCell<Self>>, peer_id: PeerID) {
        spawn_local(async move {
            if let Err(e) = Driver::execute(
                driver,
                Input::<Msg>::Handshake {
                    from: peer_id,
                    event: HandshakeInput::ConnectionDropped,
                },
            )
            .await
            {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "ICE-driven ConnectionDropped propagation failed: {e:#}"
                )));
            }
        });
    }

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

        let sdp = conn.create_answer(&offer.sdp).await?;

        let outputs = driver.borrow_mut().fsm.process(Input::<Msg>::Handshake {
            from: peer_id.clone(),
            event: HandshakeInput::AnswerCreated(sdp),
        })?;

        driver.borrow_mut().connections.insert(peer_id, conn);

        Ok(outputs)
    }

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

        conn.accept_answer(&answer.sdp).await?;

        Ok(vec![])
    }

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

    fn execute_connected(&mut self) -> Result<Vec<Output<Msg>>> {
        self.callbacks.borrow().emit(EventType::Connected)?;
        Ok(vec![])
    }

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

    fn attach_data_channel_callbacks(
        peer_id: PeerID,
        driver: Rc<RefCell<Self>>,
        dc_manager: &mut DataChannelManager,
    ) -> Result<()> {
        {
            let peer_id = peer_id.clone();
            let driver = driver.clone();
            dc_manager.setup_on_open(move || {
                let peer_id = peer_id.clone();
                let driver = driver.clone();

                spawn_local(async move {
                    if let Err(e) = Driver::execute(
                        driver,
                        Input::<Msg>::Handshake {
                            from: peer_id,
                            event: HandshakeInput::DataChannelOpen,
                        },
                    )
                    .await
                    {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Error while routing DataChannelOpen: {:?}",
                            e
                        )));
                    }
                });
            });
        }

        {
            let peer_id = peer_id.clone();
            let driver = driver.clone();
            dc_manager.setup_on_message(move |data| {
                let data: MsgPayload<Msg> = match serde_json::from_slice(&data) {
                    Ok(data) => data,
                    Err(err) => {
                        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                            "Failed to deserialize incoming message: {err:#}"
                        )));
                        return;
                    }
                };
                let peer_id = peer_id.clone();
                let driver = driver.clone();
                spawn_local(async move {
                    if let Err(err) = Driver::execute(
                        driver,
                        Input::<Msg>::MessageReceived {
                            peer_from: peer_id,
                            data,
                        },
                    )
                    .await
                    {
                        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                            "Failed to route incoming message: {err:#}"
                        )));
                    }
                });
            });
        }

        {
            let peer_id = peer_id.clone();
            let driver = driver.clone();
            dc_manager.setup_on_close(move || {
                let peer_id = peer_id.clone();
                let driver = driver.clone();
                spawn_local(async move {
                    if let Err(e) = Driver::execute(
                        driver,
                        Input::<Msg>::Handshake {
                            from: peer_id,
                            event: HandshakeInput::ConnectionDropped,
                        },
                    )
                    .await
                    {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Error while routing ConnectionDropped: {:?}",
                            e
                        )));
                    }
                });
            });
        }

        Ok(())
    }
}
