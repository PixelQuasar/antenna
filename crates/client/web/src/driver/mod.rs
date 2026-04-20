use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use antenna_protocol::{
    HandshakeMode, HandshakeStrategy, Input, MeshNodeFSM, MsgPayload, Output, PeerID,
    UserMsgPayload,
};
use anyhow::{Context, Result};

use crate::{
    EXECUTE_FUEL,
    utils::{Dispatcher, IceServerConfig, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_handshake;

pub struct Driver<Msg>
where
    Msg: UserMsgPayload,
{
    /// SansIO-based protocol finite state machine to handle main logic
    fsm: Rc<RefCell<MeshNodeFSM>>,

    /// Weak self-like shared handle for callbacks
    self_ref: Option<Rc<RefCell<Driver<Msg>>>>,

    /// Map of JS RTC peer connection wrappers
    pc_managers: HashMap<PeerID, PeerConnectionManager>,

    /// Map of JS RTC data channel wrappers
    dc_managers: HashMap<PeerID, Rc<RefCell<Option<DataChannelManager>>>>,

    /// ICE servers configuration
    ice_servers: Vec<IceServerConfig>,

    /// RTC callbacks set that are invoked on webRTC events
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl<Msg> Driver<Msg>
where
    Msg: UserMsgPayload,
{
    pub fn new(
        ice_servers: Vec<IceServerConfig>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    ) -> Self {
        Self {
            fsm: Rc::new(RefCell::new(MeshNodeFSM::new())),
            self_ref: None,
            pc_managers: HashMap::new(),
            dc_managers: HashMap::new(),
            ice_servers,
            callbacks,
        }
    }

    pub fn attach_self(&mut self, self_ref: Rc<RefCell<Driver<Msg>>>) {
        self.self_ref = Some(self_ref);
    }

    pub fn self_handle(&self) -> Result<Rc<RefCell<Driver<Msg>>>> {
        self.self_ref
            .as_ref()
            .cloned()
            .context("Driver self handle is not attached")
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.fsm.borrow().is_connected(peer)
    }

    pub fn connected_peers(&self) -> HashSet<PeerID> {
        self.fsm.borrow().connected_peers()
    }

    pub async fn execute(&mut self, input: Input<Msg>) -> Result<()> {
        let mut queue = VecDeque::from(self.fsm.borrow_mut().process(input)?);

        let mut fuel = EXECUTE_FUEL;
        while let Some(output) = queue.pop_back()
            && fuel > 0
        {
            fuel -= 1;
            let outputs = match output {
                Output::Handshake { peer, event } => self.execute_handshake(&peer, event).await?,
                Output::SendMessage { peer_to, data } => self.send(&peer_to, &data)?,
                Output::ReceiveMessage { peer_from, data } => {
                    match data {
                        MsgPayload::User(data) => self
                            .callbacks
                            .borrow()
                            .emit(RtcEvent::UserMessage(peer_from, data))?,
                        _ => {}
                    };
                    vec![]
                }
                Output::PeerConnected { peer } => {
                    self.callbacks
                        .borrow()
                        .emit(RtcEvent::PeerConnected(peer))?;
                    vec![]
                }
                Output::PeerDisconnected { peer } => {
                    self.callbacks
                        .borrow()
                        .emit(RtcEvent::PeerDisconnected(peer))?;
                    vec![]
                }
                Output::PeerAppeared { .. } => vec![],
            };
            queue.extend(outputs);
        }

        Ok(())
    }

    pub fn send(&self, peer: &PeerID, data: &MsgPayload<Msg>) -> Result<Vec<Output<Msg>>> {
        let dc = self.dc_managers.get(peer).context("Peer not found")?;
        if let Some(dc) = dc.borrow().as_ref() {
            dc.send_data(data)?;
        }
        Ok(vec![])
    }

    pub fn fsm(&self) -> Rc<RefCell<MeshNodeFSM>> {
        self.fsm.clone()
    }

    pub async fn init_host(&mut self, peer_id: PeerID) -> Result<()> {
        self.execute(Input::InitHandshake {
            with: peer_id,
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Host,
        })
        .await
    }

    pub async fn init_joiner(&mut self, peer_id: PeerID) -> Result<()> {
        self.execute(Input::InitHandshake {
            with: peer_id,
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Joiner,
        })
        .await
    }

    pub fn id(&self) -> PeerID {
        self.fsm.borrow().id().clone()
    }
}
