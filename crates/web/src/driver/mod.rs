use std::{cell::RefCell, collections::HashMap, rc::Rc};

use antenna_protocol::{Input, MeshNodeFSM, Output, PeerID};
use anyhow::{Context, Result};

use crate::{
    utils::{Dispatcher, IceServerConfig, Msg, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_handshake;

pub struct Driver {
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

impl Driver {
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

    pub async fn process_input(
        &mut self,
        input: Input<Msg>,
    ) -> Result<(Vec<Output<Msg>>, Option<String>)> {
        let was_connected = !self.fsm.borrow().connected_peers().is_empty();
        let outputs = self.fsm.borrow_mut().process(input);

        let mut unhandled = Vec::new();
        let mut sdp = None;

        for output in outputs {
            match output {
                Output::Handshake { peer, event } => {
                    if let Some(local_sdp) = self.execute_handshake(&peer, event).await? {
                        sdp = Some(local_sdp);
                    }
                }
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
                    .emit(RtcEvent::Message(peer_from, data)),
                Output::PeerConnected { peer } => {
                    self.callbacks.borrow().emit(RtcEvent::PeerConnected(peer));
                }
                Output::PeerDisconnected { peer } => {
                    self.callbacks
                        .borrow()
                        .emit(RtcEvent::PeerDisconnected(peer));
                }
                other => unhandled.push(other),
            }
        }

        let is_connected = !self.fsm.borrow().connected_peers().is_empty();
        if !was_connected && is_connected {
            self.callbacks.borrow().emit(RtcEvent::Connected);
        } else if was_connected && !is_connected {
            self.callbacks.borrow().emit(RtcEvent::Disconnected);
        }

        Ok((unhandled, sdp))
    }

    async fn send(&self, peer: &PeerID, data: &[u8]) -> Result<()> {
        let dc = self.dc_managers.get(peer).context("Peer not found")?;
        if let Some(dc) = dc.borrow().as_ref() {
            dc.send_data(data)?;
        }
        Ok(())
    }

    async fn broadcast(&self, data: &[u8]) -> Result<()> {
        for (_, dc) in &self.dc_managers {
            if let Some(dc) = dc.borrow().as_ref() {
                dc.send_data(data)?;
            }
        }
        Ok(())
    }
}
