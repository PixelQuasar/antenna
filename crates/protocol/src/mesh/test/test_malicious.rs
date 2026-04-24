#[cfg(test)]
mod test {
    use crate::{Identity, SignalingPayload};

    #[test]
    fn sdp_substitution_is_rejected() {
        let legitimate = Identity::new();

        let real_sdp = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n".to_string();
        let token = legitimate.create_token(&real_sdp).unwrap();

        let tampered = SignalingPayload {
            sdp: "v=0\r\no=- 9 9 IN IP4 attacker\r\n".to_string(),
            pubkey: legitimate.pubkey(),
            token,
        };
        let sender_id = tampered.peer_id();

        assert!(
            legitimate.verify(&tampered, &sender_id).is_err(),
            "verify must reject a payload whose SDP was substituted after signing"
        );
    }

    #[test]
    fn valid_payload_is_accepted() {
        let id = Identity::new();

        let sdp = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n".to_string();
        let token = id.create_token(&sdp).unwrap();

        let payload = SignalingPayload {
            token,
            sdp,
            pubkey: id.pubkey(),
        };
        let sender_id = payload.peer_id();

        assert!(id.verify(&payload, &sender_id).is_ok());
    }
}
