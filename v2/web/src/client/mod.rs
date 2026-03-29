// v2/web/src/client/mod.rs

use std::{cell::RefCell, rc::Rc};

use crate::{
    driver::Driver,
    utils::{
        IceServerConfig, MessageCallback, Msg, PeerConnectedCallback, PeerDisconnectedCallback,
        RtcCallbacks,
    },
};
use antenna_protocol::{Input, Output, PeerID, RelayPayload, TransportInput};
use anyhow::Result;

pub struct Client {
    my_id: PeerID,
    driver: Driver,
    callbacks: Rc<RefCell<RtcCallbacks<Msg>>>,
}

impl Client {
    pub fn new(my_id: PeerID) -> Self {
        Self::with_ice_servers(my_id, IceServerConfig::default_stun())
    }

    pub fn with_ice_servers(my_id: PeerID, ice_servers: Vec<IceServerConfig>) -> Self {
        let callbacks = Rc::new(RefCell::new(RtcCallbacks::default()));
        let driver = Driver::new(my_id.clone(), ice_servers, callbacks.clone());

        Self {
            my_id,
            driver,
            callbacks,
        }
    }

    pub fn my_id(&self) -> &PeerID {
        &self.my_id
    }

    pub async fn start_with_peer(
        &mut self,
        peer_id: PeerID,
    ) -> Result<(String, Vec<RelayMessage>)> {
        let outputs = self
            .driver
            .process_input(Input::Transport {
                peer: peer_id.clone(),
                event: TransportInput::InitNegotiation,
            })
            .await?;

        let sdp = extract_local_sdp(&peer_id, &self.driver)?;
        let relays = extract_relays(&outputs);

        Ok((sdp, relays))
    }

    pub async fn receive_offer(
        &mut self,
        peer_id: PeerID,
        offer_sdp: String,
    ) -> Result<(String, Vec<RelayMessage>)> {
        let outputs = self
            .driver
            .process_input(Input::Transport {
                peer: peer_id.clone(),
                event: TransportInput::SDPOfferReceived { sdp: offer_sdp },
            })
            .await?;

        let sdp = extract_local_sdp(&peer_id, &self.driver)?;
        let relays = extract_relays(&outputs);

        Ok((sdp, relays))
    }

    pub async fn receive_answer(
        &mut self,
        peer_id: PeerID,
        answer_sdp: String,
    ) -> Result<Vec<RelayMessage>> {
        let outputs = self
            .driver
            .process_input(Input::Transport {
                peer: peer_id,
                event: TransportInput::SDPAnswerReceived { sdp: answer_sdp },
            })
            .await?;

        Ok(extract_relays(&outputs))
    }

    pub async fn process_relay(
        &mut self,
        from: PeerID,
        payload: RelayPayload,
    ) -> Result<Vec<RelayMessage>> {
        let outputs = self
            .driver
            .process_input(Input::RelayReceived { from, payload })
            .await?;

        Ok(extract_relays(&outputs))
    }

    pub async fn send_to(&mut self, peer_id: PeerID, data: Msg) -> Result<()> {
        self.driver
            .process_input(Input::PeerSend {
                peer_to: peer_id,
                data: data,
            })
            .await?;
        Ok(())
    }

    pub async fn broadcast(&mut self, data: Msg) -> Result<()> {
        self.driver
            .process_input(Input::PeerBroadcast {
                data: data.to_vec(),
            })
            .await?;
        Ok(())
    }

    pub fn is_connected(&self, peer_id: PeerID) -> bool {
        self.driver.is_connected(&peer_id)
    }

    pub fn connected_peers(&self) -> Vec<String> {
        self.driver
            .connected_peers()
            .into_iter()
            .map(|p| p.to_string())
            .collect()
    }

    pub fn set_on_message(&mut self, cb: MessageCallback<Msg>) {
        self.callbacks.borrow_mut().on_message = Some(cb);
    }

    pub fn set_on_peer_connected(&mut self, cb: PeerConnectedCallback) {
        self.callbacks.borrow_mut().on_peer_connected = Some(cb);
    }

    pub fn set_on_peer_disconnected(&mut self, cb: PeerDisconnectedCallback) {
        self.callbacks.borrow_mut().on_peer_disconnected = Some(cb);
    }

    pub fn set_js_on_message(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_message = Some(cb)
    }

    pub fn set_js_on_connected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_connected = Some(cb)
    }

    pub fn set_js_on_disconnected(&mut self, cb: js_sys::Function) {
        self.callbacks.borrow_mut().js_on_disconnected = Some(cb)
    }
}

// Helper types
#[derive(Clone, Debug)]
pub struct RelayMessage {
    pub via: PeerID,
    pub payload: RelayPayload,
}

// Helper functions
fn extract_relays(outputs: &[Output<Msg>]) -> Vec<RelayMessage> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::Relay { via, payload } => Some(RelayMessage {
                via: via.clone(),
                payload: payload.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn extract_local_sdp(peer: &PeerID, driver: &Driver) -> Result<String> {
    Err(anyhow::anyhow!("local_sdp extraction not implemented yet"))
}
