use crate::{
    driver::Driver,
    utils::{Dispatcher, Msg, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};
use antenna_protocol::{ClientFSM, Input, Output, TransportFSM, TransportInput, TransportOutput};
use anyhow::{Context, Result};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

impl<T: TransportFSM + 'static> Driver<T> {
    pub(crate) async fn execute_transport(&mut self, output: TransportOutput) -> Result<()> {
        match output {
            TransportOutput::InitSDPOffer => self.execute_init_offer::<Msg>().await,
            TransportOutput::InitSDPAnswer { offer_sdp } => {
                self.execute_init_answer::<Msg>(offer_sdp).await
            }
            TransportOutput::AcceptSDPAnswer { sdp } => self.execute_accept_answer(sdp).await,
            TransportOutput::Close => self.execute_close(),
        }
    }

    /// Method that is called when FSM sends InitSDPOffer action to driver (when currect
    /// client is host and needs to receive answer from joiner).
    /// This method does 2 things: initiates PeerConnectionManager with SDP offer,
    /// and then set up DataChannelManager with its callbacks, bounding them to driver callbacks
    async fn execute_init_offer<Msg>(&mut self) -> Result<()> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_host_data_channel::<Msg>(pc_manager.peer_connection());

        let offer_sdp = pc_manager.create_offer().await?;
        pc_manager.set_local_description(&offer_sdp, true).await?;

        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        self.fsm
            .borrow_mut()
            .process(Input::<Msg>::Transport(TransportInput::SDPOfferCreated {
                sdp: full_sdp,
            }));

        self.pc_manager = Some(pc_manager);
        Ok(())
    }

    /// Method that is called when FSM sends InitSDPAnswer action to driver (when currect
    /// client is joiner and needs to receive answer from host).
    /// This method does 2 things: initiates PeerConnectionManager with SDP offer,
    /// and then set up DataChannelManager with its callbacks, bounding them to driver callbacks
    async fn execute_init_answer<Msg>(&mut self, offer_sdp: String) -> Result<()> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_joiner_data_channel::<Msg>(pc_manager.peer_connection());

        pc_manager.set_remote_description(&offer_sdp, true).await?;
        let answer_sdp = pc_manager.create_answer().await?;
        pc_manager.set_local_description(&answer_sdp, false).await?;
        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        self.fsm
            .borrow_mut()
            .process(Input::<Msg>::Transport(TransportInput::SDPAnswerCreated {
                sdp: full_sdp,
            }));

        self.pc_manager = Some(pc_manager);
        Ok(())
    }

    /// Method that is called when FSM sends AcceptSDPAnswer action to driver.
    /// This happens when Host receives SDP answer from Joiner.
    /// Sets the remote description on the existing PeerConnection.
    async fn execute_accept_answer(&mut self, sdp: String) -> Result<()> {
        let pc_manager = self
            .pc_manager
            .as_ref()
            .context("PeerConnection not initialized")?;

        pc_manager.set_remote_description(&sdp, false).await?;
        Ok(())
    }

    fn execute_close(&mut self) -> Result<()> {
        let dc_manager = self.dc_manager.borrow();
        if let Some(dc_manager) = dc_manager.as_ref() {
            dc_manager.close();
        }
        if let Some(pc_manager) = &self.pc_manager {
            pc_manager.close();
        }
        Ok(())
    }

    fn setup_host_data_channel<Msg>(&mut self, pc: &web_sys::RtcPeerConnection) {
        let dc_manager = DataChannelManager::new(pc, "data"); // pc.createDataChannel()
        Self::attach_data_channel_callbacks(self.fsm.clone(), self.callbacks.clone(), &dc_manager);
        *self.dc_manager.borrow_mut() = Some(dc_manager);
    }

    fn setup_joiner_data_channel<Msg>(&self, pc: &web_sys::RtcPeerConnection) {
        let fsm = self.fsm.clone();
        let callbacks = self.callbacks.clone();
        let dc_storage = self.dc_manager.clone(); // Rc<RefCell<Option<DataChannelManager>>>

        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let event: web_sys::RtcDataChannelEvent = evt.unchecked_into();
            let channel = event.channel();
            let dc_manager = DataChannelManager::from_existing(channel);

            Self::attach_data_channel_callbacks(fsm.clone(), callbacks.clone(), &dc_manager);

            *dc_storage.borrow_mut() = Some(dc_manager);
        }));

        pc.set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    /// Helper method to init data channel of current driver
    /// Creates DataChannelManager object and binds its callbacks:
    /// on_open, on_message and on_close to current driver logic.
    fn attach_data_channel_callbacks(
        fsm: Rc<RefCell<ClientFSM<T>>>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
        dc_manager: &DataChannelManager,
    ) {
        {
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_open(move || {
                fsm.borrow_mut()
                    .process(Input::<Msg>::Transport(TransportInput::DataChannelOpen));
                callbacks.borrow().emit(RtcEvent::Connected);
            });
        }

        {
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_message(move |data| {
                let output = fsm
                    .borrow_mut()
                    .process(Input::MessageReceived { peer_from: 0, data });
                if let Some(Output::ReceiveMessage { data, .. }) = output {
                    callbacks.borrow().emit(RtcEvent::Message(data));
                }
            });
        }

        {
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_close(move || {
                fsm.borrow_mut()
                    .process(Input::<Msg>::Transport(TransportInput::Disconnected));
                callbacks.borrow().emit(RtcEvent::Disconnected);
            });
        }
    }
}
