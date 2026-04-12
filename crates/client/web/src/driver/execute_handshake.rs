// v2/web/src/driver/execute_handshake.rs

use crate::{
    driver::Driver,
    webrtc::{DataChannelManager, PeerConnectionManager},
};
use antenna_protocol::{
    HandshakeInput, HandshakeOutput, Input, MsgPayload, Output, PeerID, SignalingPayload,
    UserMsgPayload,
};

use anyhow::{Context, Result};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

impl<Msg> Driver<Msg>
where
    Msg: UserMsgPayload,
{
    pub(crate) async fn execute_handshake(
        &mut self,
        peer: &PeerID,
        output: HandshakeOutput,
    ) -> Result<()> {
        match output {
            HandshakeOutput::InitSDPOffer => self.execute_init_offer(peer).await,
            HandshakeOutput::RequestSDPAnswer { offer } => {
                self.execute_init_answer(peer, offer).await
            }
            HandshakeOutput::AcceptSDPAnswer { answer } => {
                self.execute_accept_answer(peer, answer).await?;
                Ok(())
            }
            HandshakeOutput::Close => {
                self.execute_close(peer)?;
                Ok(())
            }
        }
    }

    async fn execute_init_offer(&mut self, peer: &PeerID) -> Result<()> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_host_data_channel(peer, pc_manager.peer_connection())?;

        let offer_sdp = pc_manager.create_offer().await?;
        pc_manager.set_local_description(&offer_sdp, true).await?;

        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        let outputs = self.fsm.borrow_mut().process(Input::<Msg>::Handshake {
            from: peer.clone(),
            event: HandshakeInput::SignalingCreated(SignalingPayload::Offer(full_sdp.clone())),
        })?;

        self.pc_managers.insert(peer.clone(), pc_manager);

        for output in outputs {
            match output {
                Output::SendMessage { peer_to, data } => self.send(&peer_to, &data).await?,
                _ => {}
            }
        }

        Ok(())
    }

    async fn execute_init_answer(&mut self, peer: &PeerID, offer_sdp: String) -> Result<()> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_joiner_data_channel(peer, pc_manager.peer_connection())?;

        pc_manager.set_remote_description(&offer_sdp, true).await?;
        let answer_sdp = pc_manager.create_answer().await?;
        pc_manager.set_local_description(&answer_sdp, false).await?;
        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        let outputs = self.fsm.borrow_mut().process(Input::<Msg>::Handshake {
            from: peer.clone(),
            event: HandshakeInput::SignalingCreated(SignalingPayload::Answer(full_sdp.clone())),
        })?;

        self.pc_managers.insert(peer.clone(), pc_manager);

        for output in outputs {
            match output {
                Output::SendMessage { peer_to, data } => self.send(&peer_to, &data).await?,
                _ => {}
            }
        }

        Ok(())
    }

    async fn execute_accept_answer(&mut self, peer: &PeerID, sdp: String) -> Result<()> {
        let pc_manager = self
            .pc_managers
            .get(peer)
            .context("PeerConnection not found for peer")?;

        pc_manager.set_remote_description(&sdp, false).await?;
        Ok(())
    }

    fn execute_close(&mut self, peer: &PeerID) -> Result<()> {
        if let Some(dc_ref) = self.dc_managers.get(peer) {
            if let Some(dc) = dc_ref.borrow().as_ref() {
                dc.close();
            }
        }
        if let Some(pc) = self.pc_managers.get(peer) {
            pc.close();
        }
        self.pc_managers.remove(peer);
        self.dc_managers.remove(peer);
        Ok(())
    }

    fn setup_host_data_channel(
        &mut self,
        peer: &PeerID,
        pc: &web_sys::RtcPeerConnection,
    ) -> Result<()> {
        let dc_manager = DataChannelManager::new(pc, "data");
        let driver = self.self_handle()?;
        Self::attach_data_channel_callbacks(peer.clone(), driver, &dc_manager)?;

        let dc = Rc::new(RefCell::new(Some(dc_manager)));
        self.dc_managers.insert(peer.clone(), dc);

        Ok(())
    }

    fn setup_joiner_data_channel(
        &mut self,
        peer: &PeerID,
        pc: &web_sys::RtcPeerConnection,
    ) -> Result<()> {
        let peer_id = peer.clone();
        let driver = self.self_handle()?;
        let dc_storage = Rc::new(RefCell::new(None));
        self.dc_managers.insert(peer.clone(), dc_storage.clone());
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let event: web_sys::RtcDataChannelEvent = evt.unchecked_into();
            let channel = event.channel();
            let dc_manager = DataChannelManager::from_existing(channel);

            let result =
                Self::attach_data_channel_callbacks(peer_id.clone(), driver.clone(), &dc_manager);
            if let Err(e) = result {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Error while attaching data channel callbacks: {:?}",
                    e
                )));
            };
            *dc_storage.borrow_mut() = Some(dc_manager);
        }));
        pc.set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        Ok(())
    }

    fn attach_data_channel_callbacks(
        peer: PeerID,
        driver: Rc<RefCell<Driver<Msg>>>,
        dc_manager: &DataChannelManager,
    ) -> Result<()> {
        {
            let peer = peer.clone();
            let driver = driver.clone();
            dc_manager.setup_on_open(move || {
                let peer = peer.clone();
                let driver = driver.clone();
                spawn_local(async move {
                    if let Err(e) = driver
                        .borrow_mut()
                        .process_input(Input::<Msg>::Handshake {
                            from: peer,
                            event: HandshakeInput::DataChannelOpen,
                        })
                        .await
                    {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Error while routing DataChannelOpen through driver: {:?}",
                            e
                        )));
                    }
                });
            });
        }

        {
            let peer = peer.clone();
            let driver = driver.clone();
            dc_manager.setup_on_message(move |data| {
                let peer = peer.clone();
                let driver = driver.clone();
                spawn_local(async move {
                    let data: MsgPayload<Msg> = match serde_json::from_slice(&data) {
                        Ok(data) => data,
                        Err(err) => {
                            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                                "Failed to deserialize incoming message: {err:#}"
                            )));
                            return;
                        }
                    };

                    if let Err(err) = driver
                        .borrow_mut()
                        .process_input(Input::MessageReceived {
                            peer_from: peer,
                            data,
                        })
                        .await
                    {
                        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                            "Failed to route incoming message through driver: {err:#}"
                        )));
                    }
                });
            });
        }

        {
            let peer = peer.clone();
            let driver = driver.clone();
            dc_manager.setup_on_close(move || {
                let peer = peer.clone();
                let driver = driver.clone();
                spawn_local(async move {
                    if let Err(e) = driver
                        .borrow_mut()
                        .process_input(Input::<Msg>::Handshake {
                            from: peer,
                            event: HandshakeInput::Disconnected,
                        })
                        .await
                    {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Error while routing Disconnected through driver: {:?}",
                            e
                        )));
                    }
                });
            });
        }
        Ok(())
    }
}
