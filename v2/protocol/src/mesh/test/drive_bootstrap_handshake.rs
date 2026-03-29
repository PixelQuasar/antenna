use crate::{Input, MeshFSM, Output, TransportInput, TransportOutput};

/// Drives a complete bootstrap handshake between two peers.
pub(crate) fn drive_bootstrap_handshake<Msg>(
    host: &mut MeshFSM,
    joiner: &mut MeshFSM,
) -> Vec<Output<Msg>> {
    let host_id = host.id().clone();
    let joiner_id = joiner.id().clone();

    let out = host.process::<Msg>(Input::Transport {
        peer: joiner_id.clone(),
        event: TransportInput::InitNegotiation,
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Transport {
            event: TransportOutput::InitSDPOffer,
            ..
        }
    )));

    host.process::<Msg>(Input::Transport {
        peer: joiner_id.clone(),
        event: TransportInput::SDPOfferCreated {
            sdp: "offer".into(),
        },
    });

    let out = joiner.process::<Msg>(Input::Transport {
        peer: host_id.clone(),
        event: TransportInput::SDPOfferReceived {
            sdp: "offer".into(),
        },
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Transport {
            event: TransportOutput::InitSDPAnswer { .. },
            ..
        }
    )));

    joiner.process::<Msg>(Input::Transport {
        peer: host_id.clone(),
        event: TransportInput::SDPAnswerCreated {
            sdp: "answer".into(),
        },
    });

    let out = host.process::<Msg>(Input::Transport {
        peer: joiner_id.clone(),
        event: TransportInput::SDPAnswerReceived {
            sdp: "answer".into(),
        },
    });
    assert!(out.iter().any(|o| matches!(
        o,
        Output::Transport {
            event: TransportOutput::AcceptSDPAnswer { .. },
            ..
        }
    )));

    joiner.process::<Msg>(Input::Transport {
        peer: host_id.clone(),
        event: TransportInput::DataChannelOpen,
    });

    let outputs = host.process::<Msg>(Input::Transport {
        peer: joiner_id.clone(),
        event: TransportInput::DataChannelOpen,
    });

    assert!(host.is_connected(&joiner_id));
    assert!(joiner.is_connected(&host_id));

    outputs
}
