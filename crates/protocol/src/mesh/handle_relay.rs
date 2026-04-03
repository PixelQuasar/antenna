use crate::{
    HandshakeContext, HandshakeFSM, HandshakeInput, Host, Joiner, MeshNodeFSM, Output, PeerID,
    RelayPayload,
};

impl MeshNodeFSM {
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
                let mut handshake = HandshakeFSM::Host(Host::new());
                let mut out = vec![];
                if let Some(event) = handshake.process(HandshakeInput::InitNegotiation) {
                    out.push(Output::Handshake {
                        peer: peer.clone(),
                        event,
                    });
                }
                self.handshakes.insert(
                    peer,
                    HandshakeContext {
                        handshake,
                        via: Some(from),
                    },
                );
                out
            }
            RelayPayload::HandshakeForward { src, dst, event } => {
                if dst != self.id {
                    if self.connected.contains(&dst) {
                        vec![Output::Relay {
                            via: dst.clone(),
                            payload: RelayPayload::HandshakeForward { src, dst, event },
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    if !self.handshakes.contains_key(&src) {
                        let handshake = match &event {
                            HandshakeInput::SDPOfferReceived { .. } => {
                                HandshakeFSM::Joiner(Joiner::new())
                            }
                            _ => return vec![],
                        };
                        self.handshakes.insert(
                            src.clone(),
                            HandshakeContext {
                                handshake,
                                via: Some(from),
                            },
                        );
                    }
                    self.handle_handshake(src, event)
                }
            }
        }
    }
}
