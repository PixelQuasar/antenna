#[cfg(test)]
mod test {
    use crate::{Identity, SignalingPayload};

    #[test]
    fn tampered_token_is_rejected() {
        let legitimate = Identity::new();

        let real_sdp = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n";
        let token = legitimate.create_token(real_sdp).unwrap();

        let mid = token.len() / 2;
        let mut fake_token = token.clone();
        fake_token[mid] ^= 0xFF;

        let tampered = SignalingPayload {
            pubkey: legitimate.pubkey(),
            token: fake_token,
        };
        let sender_id = tampered.peer_id();

        assert!(
            legitimate.verify(&tampered, &sender_id).is_err(),
            "verify must reject a payload whose token bytes were tampered with"
        );
    }

    #[test]
    fn pubkey_mismatch_is_rejected() {
        let legitimate = Identity::new();
        let attacker = Identity::new();

        let real_sdp = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n";
        let token = legitimate.create_token(real_sdp).unwrap();

        let forged = SignalingPayload {
            pubkey: attacker.pubkey(),
            token,
        };
        let claimed_sender = forged.peer_id();

        assert!(
            legitimate.verify(&forged, &claimed_sender).is_err(),
            "verify must reject a payload whose pubkey doesn't match the token's signer"
        );
    }

    #[test]
    fn valid_payload_returns_signed_sdp() {
        let id = Identity::new();

        let real_sdp = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n".to_string();
        let token = id.create_token(&real_sdp).unwrap();

        let payload = SignalingPayload {
            token,
            pubkey: id.pubkey(),
        };
        let sender_id = payload.peer_id();

        let extracted = id.verify(&payload, &sender_id).unwrap();
        assert_eq!(extracted, real_sdp);
    }
}
