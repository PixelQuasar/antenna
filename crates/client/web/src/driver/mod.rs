use std::{cell::RefCell, collections::HashMap, rc::Rc};

use antenna_protocol::{Input, MeshNodeFSM, Output, PeerID};
use antenna_shared::AntennaPayload;
use anyhow::{Context, Result};

use crate::{
    utils::{Dispatcher, IceServerConfig, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_handshake;

pub struct Driver<Msg>
where
    Msg: AntennaPayload,
{
    /// SansIO-based protocol finite state machine to handle main logic
    fsm: Rc<RefCell<MeshNodeFSM>>,

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
    Msg: AntennaPayload,
{
    pub fn new(
        id: PeerID,
        ice_servers: Vec<IceServerConfig>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    ) -> Self {
        Self {
            fsm: Rc::new(RefCell::new(MeshNodeFSM::new(id))),
            pc_managers: HashMap::new(),
            dc_managers: HashMap::new(),
            ice_servers,
            callbacks,
        }
    }

    pub fn is_connected(&self, peer: &PeerID) -> bool {
        self.fsm.borrow().is_connected(peer)
    }

    pub fn connected_peers(&self) -> Vec<PeerID> {
        self.fsm
            .borrow()
            .connected_peers()
            .iter()
            .cloned()
            .collect()
    }

    pub async fn process_input(&mut self, input: Input<Msg>) -> Result<Vec<Output<Msg>>> {
        let was_connected = !self.fsm.borrow().connected_peers().is_empty();
        let outputs = self.fsm.borrow_mut().process(input);

        let mut unhandled = Vec::new();

        for output in outputs {
            match output {
                Output::Handshake { peer, event } => self.execute_handshake(&peer, event).await?,
                Output::SendMessage { peer_to, data } => {
                    self.send(&peer_to, &data).await?;
                }
                Output::Broadcast { data } => {
                    self.broadcast(&data).await?;
                }
                Output::ReceiveMessage {
                    peer_from, data, ..
                } => self
                    .callbacks
                    .borrow()
                    .emit(RtcEvent::Message(peer_from, data))?,
                Output::PeerConnected { peer } => self
                    .callbacks
                    .borrow()
                    .emit(RtcEvent::PeerConnected(peer))?,
                Output::PeerDisconnected { peer } => self
                    .callbacks
                    .borrow()
                    .emit(RtcEvent::PeerDisconnected(peer))?,
                other => unhandled.push(other),
            }
        }

        let is_connected = !self.fsm.borrow().connected_peers().is_empty();
        if !was_connected && is_connected {
            self.callbacks.borrow().emit(RtcEvent::Connected)?
        } else if was_connected && !is_connected {
            self.callbacks.borrow().emit(RtcEvent::Disconnected)?
        }

        Ok(unhandled)
    }

    async fn send(&self, peer: &PeerID, data: &Msg) -> Result<()> {
        let dc = self.dc_managers.get(peer).context("Peer not found")?;
        if let Some(dc) = dc.borrow().as_ref() {
            dc.send_data(data)?;
        }
        Ok(())
    }

    async fn broadcast(&self, data: &Msg) -> Result<()> {
        for (_, dc) in &self.dc_managers {
            if let Some(dc) = dc.borrow().as_ref() {
                dc.send_data(data)?;
            }
        }
        Ok(())
    }

    pub fn fsm(&self) -> Rc<RefCell<MeshNodeFSM>> {
        self.fsm.clone()
    }
}
