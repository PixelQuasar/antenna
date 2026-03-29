use crate::{
    Host, Joiner, MeshFSM, Output, PeerID, RelayPayload, TransportContext, TransportFSM,
    TransportInput, TransportState,
};

impl MeshFSM {
    pub(crate) fn handle_transport<Msg>(
        &mut self,
        peer: PeerID,
        event: TransportInput,
    ) -> Vec<Output<Msg>> {
        if !self.handshakes.contains_key(&peer) {
            let transport = match &event {
                TransportInput::InitNegotiation => TransportFSM::Host(Host::new()),
                TransportInput::SDPOfferReceived { .. } => TransportFSM::Joiner(Joiner::new()),
                _ => return vec![],
            };

            self.handshakes.insert(
                peer.clone(),
                TransportContext {
                    transport,
                    via: None,
                },
            );
        }

        let input_sdp = match &event {
            TransportInput::SDPOfferCreated { sdp } => Some(sdp.clone()),
            TransportInput::SDPAnswerCreated { sdp } => Some(sdp.clone()),
            _ => None,
        };

        let handshake_ctx = self.handshakes.get_mut(&peer).unwrap();
        let transport_out = handshake_ctx.transport.process(event.clone());
        let state = handshake_ctx.transport.state();
        let relay_via = handshake_ctx.via.clone();

        match state {
            TransportState::WaitingForAnswer => {
                let mut out = vec![];
                if let Some(event) = transport_out {
                    out.push(Output::Transport {
                        peer: peer.clone(),
                        event,
                    });
                }
                if let (Some(via), Some(sdp)) = (relay_via, &input_sdp) {
                    out.push(Output::Relay {
                        via,
                        payload: RelayPayload::TransportForward {
                            src: self.id.clone(),
                            dst: peer,
                            event: TransportInput::SDPOfferReceived { sdp: sdp.clone() },
                        },
                    });
                }
                out
            }
            TransportState::Connected => {
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
                if let Some(event) = transport_out {
                    out.push(Output::Transport { peer, event });
                }
                out
            }
            TransportState::WaitingForDataChannel => {
                let mut out = Vec::new();
                if let Some(event) = transport_out {
                    out.push(Output::Transport {
                        peer: peer.clone(),
                        event,
                    });
                }
                if let (Some(via), Some(sdp)) = (relay_via, &input_sdp) {
                    out.push(Output::Relay {
                        via,
                        payload: RelayPayload::TransportForward {
                            src: self.id.clone(),
                            dst: peer,
                            event: TransportInput::SDPAnswerReceived { sdp: sdp.clone() },
                        },
                    });
                }
                out
            }
            TransportState::Closed => {
                self.handshakes.remove(&peer);
                let mut out = vec![Output::PeerDisconnected { peer: peer.clone() }];
                if let Some(event) = transport_out {
                    out.push(Output::Transport { peer, event });
                }
                out
            }
            _ => transport_out
                .map(|event| vec![Output::Transport { peer, event }])
                .unwrap_or_default(),
        }
    }
}
