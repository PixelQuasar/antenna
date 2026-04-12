use crate::{
    HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeStrategy, Input, MeshNodeFSM, Output,
    SignalingPayload, UserMsgPayload,
};

/// Drives a complete bootstrap handshake between two peers.
pub(crate) fn drive_bootstrap_handshake<Msg: UserMsgPayload>(
    host: &mut MeshNodeFSM,
    joiner: &mut MeshNodeFSM,
) -> Vec<Output<Msg>> {
    let host_id = host.id().clone();
    let joiner_id = joiner.id().clone();

    // Create handshake FSMs
    host.process::<Msg>(Input::InitHandshake {
        with: joiner_id.clone(),
        mode: HandshakeMode::Bootstrap,
        strategy: HandshakeStrategy::Host,
    })
    .unwrap();

    joiner
        .process::<Msg>(Input::InitHandshake {
            with: host_id.clone(),
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Joiner,
        })
        .unwrap();

    // Host: Init → CreatingOffer
    let out = host
        .process::<Msg>(Input::Handshake {
            from: joiner_id.clone(),
            event: HandshakeInput::Init,
        })
        .unwrap();
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::InitSDPOffer,
            ..
        }
    )));

    // Host: SignalingCreated(Offer) → WaitingForAnswer
    host.process::<Msg>(Input::Handshake {
        from: joiner_id.clone(),
        event: HandshakeInput::SignalingCreated(SignalingPayload::Offer("offer".into())),
    })
    .unwrap();

    // Joiner: Signaling(Offer) → CreatingAnswer
    let out = joiner
        .process::<Msg>(Input::Handshake {
            from: host_id.clone(),
            event: HandshakeInput::Signaling(SignalingPayload::Offer("offer".into())),
        })
        .unwrap();
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::RequestSDPAnswer { .. },
            ..
        }
    )));

    // Joiner: SignalingCreated(Answer) → WaitingForDataChannel
    joiner
        .process::<Msg>(Input::Handshake {
            from: host_id.clone(),
            event: HandshakeInput::SignalingCreated(SignalingPayload::Answer("answer".into())),
        })
        .unwrap();

    // Host: Signaling(Answer) → WaitingForDataChannel
    let out = host
        .process::<Msg>(Input::Handshake {
            from: joiner_id.clone(),
            event: HandshakeInput::Signaling(SignalingPayload::Answer("answer".into())),
        })
        .unwrap();
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::AcceptSDPAnswer { .. },
            ..
        }
    )));

    // Both: DataChannelOpen → Connected
    joiner
        .process::<Msg>(Input::Handshake {
            from: host_id.clone(),
            event: HandshakeInput::DataChannelOpen,
        })
        .unwrap();

    let outputs = host
        .process::<Msg>(Input::Handshake {
            from: joiner_id.clone(),
            event: HandshakeInput::DataChannelOpen,
        })
        .unwrap();

    assert!(host.is_connected(&joiner_id));
    assert!(joiner.is_connected(&host_id));

    outputs
}
