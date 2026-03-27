use std::{cell::RefCell, rc::Rc};

use antenna_protocol::{ClientFSM, Input, Output, TransportFSM};
use anyhow::{Context, Result};

use crate::{
    utils::{Dispatcher, IceServerConfig, Msg, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};

mod execute_transport;

pub struct Driver<T: TransportFSM + 'static> {
    /// SansIO-based protocol finite state machine to handle main logic
    fsm: Rc<RefCell<ClientFSM<T>>>,

    /// JS RTC peer connection wrapper
    pc_manager: Option<PeerConnectionManager>,

    /// JS RTC data channel wrapper
    dc_manager: Option<DataChannelManager>,

    /// ICE servers configuration
    ice_servers: Vec<IceServerConfig>,

    /// RTC callbacks set that are invoke in webRTC events
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl<T: TransportFSM + 'static> Driver<T> {
    pub fn new(
        ice_servers: Vec<IceServerConfig>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
    ) -> Self {
        Self {
            fsm: Rc::new(RefCell::new(ClientFSM::new())),
            pc_manager: None,
            dc_manager: None,
            ice_servers,
            callbacks,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.fsm.borrow().is_connected()
    }

    pub fn local_sdp(&self) -> Option<String> {
        self.fsm.borrow().local_sdp()
    }

    pub async fn process_input(&mut self, input: Input<Msg>) -> Result<()> {
        let output = self.fsm.borrow_mut().process(input);
        if let Some(output) = output {
            match output {
                Output::Transport(transport_output) => {
                    self.execute_transport::<Msg>(transport_output).await?;
                }
                Output::SendMessage { data, .. } | Output::Broadcast { data } => {
                    self.send(&data).await?;
                }
                Output::ReceiveMessage { data, .. } => {
                    self.callbacks.borrow().emit(RtcEvent::Message(data))
                }
            }
        }
        Ok(())
    }

    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let dc_manager = self
            .dc_manager
            .as_ref()
            .context("DataChannel not initialized")?;

        dc_manager.send_data(data)?;
        Ok(())
    }
}
