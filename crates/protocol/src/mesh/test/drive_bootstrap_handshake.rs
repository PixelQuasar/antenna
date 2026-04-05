use crate::{
    HandshakeInput, HandshakeMode, HandshakeOutput, Input, MeshNodeFSM, Output, UserMsgPayload,
};

/// Drives a complete bootstrap handshake between two peers.
pub(crate) fn drive_bootstrap_handshake<Msg: UserMsgPayload>(
    host: &mut MeshNodeFSM,
    joiner: &mut MeshNodeFSM,
) -> Vec<Output<Msg>> {
    let host_id = host.id().clone();
    let joiner_id = joiner.id().clone();

    let out = host.process::<Msg>(Input::Handshake {
        from: joiner_id.clone(),
        event: HandshakeInput::InitNegotiation {
            mode: HandshakeMode::Bootstrap,
        },
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::InitSDPOffer,
            ..
        }
    )));

    host.process::<Msg>(Input::Handshake {
        from: joiner_id.clone(),
        event: HandshakeInput::SDPOfferCreated {
            sdp: "offer".into(),
        },
    });

    let out = joiner.process::<Msg>(Input::Handshake {
        from: host_id.clone(),
        event: HandshakeInput::SDPOfferReceived {
            sdp: "offer".into(),
            mode: HandshakeMode::Bootstrap,
        },
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::RequestSDPAnswer { .. },
            ..
        }
    )));

    joiner.process::<Msg>(Input::Handshake {
        from: host_id.clone(),
        event: HandshakeInput::SDPAnswerCreated {
            sdp: "answer".into(),
        },
    });

    let out = host.process::<Msg>(Input::Handshake {
        from: joiner_id.clone(),
        event: HandshakeInput::SDPAnswerReceived {
            sdp: "answer".into(),
        },
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::AcceptSDPAnswer { .. },
            ..
        }
    )));

    joiner.process::<Msg>(Input::Handshake {
        from: host_id.clone(),
        event: HandshakeInput::DataChannelOpen,
    });

    let outputs = host.process::<Msg>(Input::Handshake {
        from: joiner_id.clone(),
        event: HandshakeInput::DataChannelOpen,
    });

    assert!(host.is_connected(&joiner_id));
    assert!(joiner.is_connected(&host_id));

    outputs
}
