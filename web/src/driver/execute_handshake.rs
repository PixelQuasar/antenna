// v2/web/src/driver/execute_handshake.rs

use crate::{
    driver::Driver,
    utils::{Dispatcher, Msg, RtcCallbacks, RtcEvent},
    webrtc::{DataChannelManager, PeerConnectionManager},
};
use antenna_protocol::{HandshakeInput, HandshakeOutput, Input, MeshNodeFSM, Output, PeerID};
use anyhow::{Context, Result};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

impl Driver {
    pub(crate) async fn execute_handshake(
        &mut self,
        peer: &PeerID,
        output: HandshakeOutput,
    ) -> Result<Option<String>> {
        match output {
            HandshakeOutput::InitSDPOffer => self.execute_init_offer(peer).await,
            HandshakeOutput::InitSDPAnswer { offer_sdp } => {
                self.execute_init_answer(peer, offer_sdp).await
            }
            HandshakeOutput::AcceptSDPAnswer { sdp } => {
                self.execute_accept_answer(peer, sdp).await?;
                Ok(None)
            }
            HandshakeOutput::Close => {
                self.execute_close(peer)?;
                Ok(None)
            }
        }
    }

    async fn execute_init_offer(&mut self, peer: &PeerID) -> Result<Option<String>> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_host_data_channel(peer, pc_manager.peer_connection());

        let offer_sdp = pc_manager.create_offer().await?;
        pc_manager.set_local_description(&offer_sdp, true).await?;

        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        self.fsm.borrow_mut().process(Input::<Msg>::Handshake {
            from: peer.clone(),
            event: HandshakeInput::SDPOfferCreated {
                sdp: full_sdp.clone(),
            },
        });

        self.pc_managers.insert(peer.clone(), pc_manager);
        Ok(Some(full_sdp))
    }

    async fn execute_init_answer(
        &mut self,
        peer: &PeerID,
        offer_sdp: String,
    ) -> Result<Option<String>> {
        let pc_manager = PeerConnectionManager::from_ice_config(&self.ice_servers)?;

        self.setup_joiner_data_channel(peer, pc_manager.peer_connection());

        pc_manager.set_remote_description(&offer_sdp, true).await?;
        let answer_sdp = pc_manager.create_answer().await?;
        pc_manager.set_local_description(&answer_sdp, false).await?;
        let full_sdp = pc_manager.wait_for_ice_gathering_complete().await?;

        self.fsm.borrow_mut().process(Input::<Msg>::Handshake {
            from: peer.clone(),
            event: HandshakeInput::SDPAnswerCreated {
                sdp: full_sdp.clone(),
            },
        });

        self.pc_managers.insert(peer.clone(), pc_manager);
        Ok(Some(full_sdp))
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

    fn setup_host_data_channel(&mut self, peer: &PeerID, pc: &web_sys::RtcPeerConnection) {
        let dc_manager = DataChannelManager::new(pc, "data");
        Self::attach_data_channel_callbacks(
            peer.clone(),
            self.fsm.clone(),
            self.callbacks.clone(),
            &dc_manager,
        );

        let dc = Rc::new(RefCell::new(Some(dc_manager)));
        self.dc_managers.insert(peer.clone(), dc);
    }

    fn setup_joiner_data_channel(&mut self, peer: &PeerID, pc: &web_sys::RtcPeerConnection) {
        let peer_id = peer.clone();
        let fsm = self.fsm.clone();
        let callbacks = self.callbacks.clone();
        let dc_storage = Rc::new(RefCell::new(None));
        self.dc_managers.insert(peer.clone(), dc_storage.clone());
        let cb = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |evt: JsValue| {
            let event: web_sys::RtcDataChannelEvent = evt.unchecked_into();
            let channel = event.channel();
            let dc_manager = DataChannelManager::from_existing(channel);

            Self::attach_data_channel_callbacks(
                peer_id.clone(),
                fsm.clone(),
                callbacks.clone(),
                &dc_manager,
            );
            *dc_storage.borrow_mut() = Some(dc_manager);
        }));

        pc.set_ondatachannel(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }

    fn attach_data_channel_callbacks(
        peer: PeerID,
        fsm: Rc<RefCell<MeshNodeFSM>>,
        callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
        dc_manager: &DataChannelManager,
    ) {
        {
            let peer = peer.clone();
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_open(move || {
                let was_empty = fsm.borrow().connected_peers().is_empty();

                fsm.borrow_mut().process(Input::<Msg>::Handshake {
                    from: peer.clone(),
                    event: HandshakeInput::DataChannelOpen,
                });

                if was_empty {
                    callbacks.borrow().emit(RtcEvent::Connected);
                }
                callbacks
                    .borrow()
                    .emit(RtcEvent::PeerConnected(peer.clone()));
            });
        }

        {
            let peer = peer.clone();
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_message(move |data| {
                let outputs = fsm.borrow_mut().process(Input::MessageReceived {
                    peer_from: peer.clone(),
                    data,
                });
                for output in outputs {
                    if let Output::ReceiveMessage {
                        peer_from, data, ..
                    } = output
                    {
                        callbacks.borrow().emit(RtcEvent::Message(peer_from, data));
                    }
                }
            });
        }

        {
            let peer = peer.clone();
            let fsm = fsm.clone();
            let callbacks = callbacks.clone();
            dc_manager.setup_on_close(move || {
                fsm.borrow_mut().process(Input::<Msg>::Handshake {
                    from: peer.clone(),
                    event: HandshakeInput::Disconnected,
                });

                callbacks
                    .borrow()
                    .emit(RtcEvent::PeerDisconnected(peer.clone()));

                if fsm.borrow().connected_peers().is_empty() {
                    callbacks.borrow().emit(RtcEvent::Disconnected);
                }
            });
        }
    }
}
