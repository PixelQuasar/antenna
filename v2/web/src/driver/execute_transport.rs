use antenna_protocol::{Input, Output, TransportFSM, TransportInput, TransportOutput};
use anyhow::{Context, Result};
use wasm_bindgen::prelude::*;

use crate::{
    driver::Driver,
    webrtc::{DataChannelManager, PeerConnectionManager},
};

impl<T: TransportFSM + 'static> Driver<T> {
    pub(crate) async fn execute_transport<Msg>(&mut self, output: TransportOutput) -> Result<()> {
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

        let offer_sdp = pc_manager.create_offer().await?;
        pc_manager.set_local_description(&offer_sdp, true).await?;
        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        if let Some(on_offer_ready) = self.on_offer_ready {
            on_offer_ready(full_sdp)
        }

        let _ = self
            .init_data_channel::<Msg>(pc_manager.peer_connection())
            .await;
        self.pc_manager = Some(pc_manager);
        Ok(())
    }

    /// Method that is called when FSM sends InitSDPAnswer action to driver (when currect
    /// client is joiner and needs to receive answer from host).
    /// This method does 2 things: initiates PeerConnectionManager with SDP offer,
    /// and then set up DataChannelManager with its callbacks, bounding them to driver callbacks
    async fn execute_init_answer<Msg>(&mut self, offer_sdp: String) -> Result<()> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;
        pc_manager.set_remote_description(&offer_sdp, true).await?;

        let answer_sdp = pc_manager.create_answer().await?;
        pc_manager.set_local_description(&answer_sdp, false).await?;

        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;
        if let Some(on_answer_ready) = self.on_answer_ready {
            on_answer_ready(full_sdp);
        }

        let _ = self
            .init_data_channel::<Msg>(pc_manager.peer_connection())
            .await;
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
        if let Some(dc_manager) = &self.dc_manager {
            dc_manager.close();
        }
        if let Some(pc_manager) = &self.pc_manager {
            pc_manager.close();
        }
        Ok(())
    }

    /// Helper method to init data channel of current driver
    /// Creates DataChannelManager object and binds its callbacks:
    /// on_open, on_message and on_close to current driver logic.
    async fn init_data_channel<Msg>(
        &mut self,
        peer_connection: &web_sys::RtcPeerConnection,
    ) -> Result<()> {
        let dc_manager = DataChannelManager::new(peer_connection, "data");

        {
            let fsm = self.fsm.clone();
            let on_connected = self.on_connected;
            dc_manager.setup_on_open(move || {
                fsm.borrow_mut()
                    .process(Input::<Msg>::Transport(TransportInput::DataChannelOpen));
                //if fsm.borrow().is_connected() {
                if let Some(on_connected) = on_connected {
                    on_connected();
                }
                //}
            });
        }

        {
            let fsm = self.fsm.clone();
            let on_message = self.on_message;
            dc_manager.setup_on_message(move |data| {
                let output = fsm
                    .borrow_mut()
                    .process(Input::MessageReceived { peer_from: 0, data });
                if let Some(Output::ReceiveMessage { data, .. }) = output {
                    if let Some(on_message) = on_message {
                        on_message(data);
                    }
                }
            });
        }

        {
            let fsm = self.fsm.clone();
            let on_disconnected = self.on_disconnected;
            dc_manager.setup_on_close(move || {
                fsm.borrow_mut()
                    .process(Input::<Msg>::Transport(TransportInput::Disconnected));
                if let Some(on_disconnected) = on_disconnected {
                    on_disconnected();
                }
            });
        }

        self.dc_manager = Some(dc_manager);
        Ok(())
    }
}
