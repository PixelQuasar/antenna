use crate::{
    HandshakeInput, HandshakeMode, HandshakeOutput, HandshakeStrategy, Input, MeshNodeFSM, Output,
    UserMsgPayload,
};

pub(crate) fn drive_bootstrap_handshake<Msg: UserMsgPayload>(
    host: &mut MeshNodeFSM,
    joiner: &mut MeshNodeFSM,
) -> Vec<Output<Msg>> {
    let host_id = host.id().clone();
    let joiner_id = joiner.id().clone();

    let out = host.process::<Msg>(Input::InitOpenOffer).unwrap();
    assert!(out.iter().any(|o| matches!(o, Output::InitOpenOffer)));

    host.process::<Msg>(Input::OpenOfferCreated("offer".into()))
        .unwrap();

    let offer_payload = host.metadata().offer.clone().unwrap();

    joiner
        .process::<Msg>(Input::InitHandshake {
            with: host_id.clone(),
            mode: HandshakeMode::Bootstrap,
            strategy: HandshakeStrategy::Joiner,
        })
        .unwrap();

    let out = joiner
        .process::<Msg>(Input::Handshake {
            from: host_id.clone(),
            event: HandshakeInput::Offer(offer_payload),
        })
        .unwrap();
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::RequestSDPAnswer(_),
            ..
        }
    )));

    joiner
        .process::<Msg>(Input::Handshake {
            from: host_id.clone(),
            event: HandshakeInput::AnswerCreated("answer".into()),
        })
        .unwrap();

    let answer_payload = joiner.metadata().answer.clone().unwrap();

    let out = host
        .process::<Msg>(Input::Handshake {
            from: joiner_id.clone(),
            event: HandshakeInput::Answer(answer_payload),
        })
        .unwrap();
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Handshake {
            event: HandshakeOutput::AcceptSDPAnswer(_),
            ..
        }
    )));

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
