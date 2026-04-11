use anyhow::{Result, anyhow};

use crate::{
    HandshakeInput, HandshakeOutput, HandshakeState, MeshNodeFSM, Output, PeerID, UserMsgPayload,
};

impl MeshNodeFSM {
    pub(crate) fn handle_handshake<Msg: UserMsgPayload>(
        &mut self,
        peer: PeerID,
        event: HandshakeInput,
    ) -> Result<Vec<Output<Msg>>> {
        if !self.handshakes.contains_key(&peer) {
            return Err(anyhow!("Handshake instance with peer not found"));
        }

        let ctx = self.handshakes.get_mut(&peer).unwrap();
        let handshake_out = ctx.fsm.process(event.clone())?;
        let state = ctx.fsm.state();

        if let Some(out) = handshake_out.clone() {
            match out {
                HandshakeOutput::RequestSDPAnswer { offer } => {
                    self.metadata.sdp_offer = Some(offer);
                }
                HandshakeOutput::AcceptSDPAnswer { answer } => {
                    self.metadata.sdp_answer = Some(answer);
                }
                _ => {}
            }
        }

        match state {
            HandshakeState::WaitingForAnswer => {
                let mut out: Vec<Output<Msg>> = vec![];
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake {
                        peer: peer.clone(),
                        event,
                    });
                }
                Ok(out)
            }
            HandshakeState::WaitingForDataChannel => {
                let mut out = Vec::new();
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake {
                        peer: peer.clone(),
                        event,
                    });
                }
                Ok(out)
            }
            HandshakeState::Connected => {
                self.handshakes.remove(&peer);
                self.connected.insert(peer.clone());
                let mut out = vec![];
                // out.push(Output::PeerConnected { peer: peer.clone() });
                for existing in &self.connected {
                    if existing != &peer {
                        out.push(Output::PeerAppeared {
                            peer: existing.clone(),
                        });
                    }
                }
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake { peer, event });
                }
                Ok(out)
            }
            HandshakeState::Closed => {
                self.handshakes.remove(&peer);
                let mut out = vec![Output::PeerDisconnected { peer: peer.clone() }];
                if let Some(event) = handshake_out {
                    out.push(Output::Handshake { peer, event });
                }
                Ok(out)
            }
            _ => Ok(handshake_out
                .map(|event| vec![Output::Handshake { peer, event }])
                .unwrap_or_default()),
        }
    }
}
