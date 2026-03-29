use crate::{
    Host, Joiner, MeshFSM, Output, PeerID, RelayPayload, TransportContext, TransportFSM,
    TransportInput,
};

impl MeshFSM {
    pub(crate) fn handle_relay<Msg>(
        &mut self,
        from: PeerID,
        payload: RelayPayload,
    ) -> Vec<Output<Msg>> {
        match payload {
            RelayPayload::ConnectionRequest { peer } => {
                if self.connected.contains(&peer) || self.handshakes.contains_key(&peer) {
                    return vec![];
                }
                let mut transport = TransportFSM::Host(Host::new());
                let mut out = vec![];
                if let Some(event) = transport.process(TransportInput::InitNegotiation) {
                    out.push(Output::Transport {
                        peer: peer.clone(),
                        event,
                    });
                }
                self.handshakes.insert(
                    peer,
                    TransportContext {
                        transport,
                        via: Some(from),
                    },
                );
                out
            }
            RelayPayload::TransportForward { src, dst, event } => {
                if dst != self.id {
                    if self.connected.contains(&dst) {
                        vec![Output::Relay {
                            via: dst.clone(),
                            payload: RelayPayload::TransportForward { src, dst, event },
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    if !self.handshakes.contains_key(&src) {
                        let transport = match &event {
                            TransportInput::SDPOfferReceived { .. } => {
                                TransportFSM::Joiner(Joiner::new())
                            }
                            _ => return vec![],
                        };
                        self.handshakes.insert(
                            src.clone(),
                            TransportContext {
                                transport,
                                via: Some(from),
                            },
                        );
                    }
                    self.handle_transport(src, event)
                }
            }
            RelayPayload::PeerLeft { peer } => self.handle_peer_leaving(peer),
        }
    }
}
