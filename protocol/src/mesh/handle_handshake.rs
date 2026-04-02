use crate::{
    HandshakeContext, HandshakeFSM, HandshakeInput, HandshakeState, Host, Joiner, MeshNodeFSM,
    Output, PeerID, RelayPayload,
};

impl MeshNodeFSM {
    pub(crate) fn handle_handshake<Msg>(
        &mut self,
        peer: PeerID,
        event: HandshakeInput,
    ) -> Vec<Output<Msg>> {
        if !self.handshakes.contains_key(&peer) {
            let handshake = match &event {
                HandshakeInput::InitNegotiation => HandshakeFSM::Host(Host::new()),
                HandshakeInput::SDPOfferReceived { .. } => HandshakeFSM::Joiner(Joiner::new()),
                _ => return vec![],
            };

            self.handshakes.insert(
                peer.clone(),
                HandshakeContext {
                    handshake,
                    via: None,
                },
            );
        }

        let input_sdp = match &event {
            HandshakeInput::SDPOfferCreated { sdp } => Some(sdp.clone()),
            HandshakeInput::SDPAnswerCreated { sdp } => Some(sdp.clone()),
            _ => None,
        };

        let handshake_ctx = self.handshakes.get_mut(&peer).unwrap();
        let handshake_out = handshake_ctx.handshake.process(event.clone());
        let state = handshake_ctx.handshake.state();
        let relay_via = handshake_ctx.via.clone();

        match state {
            HandshakeState::WaitingForAnswer => {
                let mut out = vec![];
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake {
                        peer: peer.clone(),
                        event,
                    });
                } else if let (Some(via), Some(sdp)) = (relay_via, &input_sdp) {
                    out.push(Output::Relay {
                        via,
                        payload: RelayPayload::HandshakeForward {
                            src: self.id.clone(),
                            dst: peer,
                            event: HandshakeInput::SDPOfferReceived { sdp: sdp.clone() },
                        },
                    });
                }
                out
            }
            HandshakeState::Connected => {
                self.handshakes.remove(&peer);
                self.connected.insert(peer.clone());
                let mut out = vec![Output::PeerConnected { peer: peer.clone() }];
                for existing in &self.connected {
                    // TODO solve problem: currently "full mesh connection" is not atomic, so new peer can start broadcasting before he is
                    // connected to anyone in mesh, that would cause race condition.
                    if existing != &peer {
                        out.push(Output::Relay {
                            via: peer.clone(),
                            payload: RelayPayload::ConnectionRequest {
                                peer: existing.clone(),
                            },
                        });
                    }
                }
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake { peer, event });
                }
                out
            }
            HandshakeState::WaitingForDataChannel => {
                let mut out = Vec::new();
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake {
                        peer: peer.clone(),
                        event,
                    });
                }
                if let (Some(via), Some(sdp)) = (relay_via, &input_sdp) {
                    out.push(Output::Relay {
                        via,
                        payload: RelayPayload::HandshakeForward {
                            src: self.id.clone(),
                            dst: peer,
                            event: HandshakeInput::SDPAnswerReceived { sdp: sdp.clone() },
                        },
                    });
                }
                out
            }
            HandshakeState::Closed => {
                self.handshakes.remove(&peer);
                let mut out = vec![Output::PeerDisconnected { peer: peer.clone() }];
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake { peer, event });
                }
                out
            }
            _ => handshake_out
                .map(|event| vec![Output::Handshake { peer, event }])
                .unwrap_or_default(),
        }
    }
}
