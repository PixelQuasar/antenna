use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use biscuit_auth::{Algorithm, KeyPair, PrivateKey, PublicKey};
use serde::Deserialize;

/// used for deserializing bytes vector to base64
pub fn deserialize_base64_vec<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    BASE64_URL_SAFE_NO_PAD
        .decode(s)
        .map_err(serde::de::Error::custom)
}

/// used to deserialize biscuit public key to base64
pub fn deserialize_base64_pubkey<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    let bytes = BASE64_URL_SAFE_NO_PAD
        .decode(s)
        .map_err(serde::de::Error::custom)?;

    PublicKey::from_bytes(&bytes, Algorithm::Ed25519).map_err(serde::de::Error::custom)
}

/// used to deserialize biscuit keypair to base64
pub fn deserialize_base64_keypair<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    let bytes = BASE64_URL_SAFE_NO_PAD
        .decode(s)
        .map_err(serde::de::Error::custom)?;

    let private =
        PrivateKey::from_bytes(&bytes, Algorithm::Ed25519).map_err(serde::de::Error::custom)?;

    Ok(KeyPair::from(&private))
}

/// used for serializing base64 to bytes vector
pub fn serialize_base64_vec<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(bytes))
}

/// used for serializing base64 to biscuit public key
pub fn serialize_base64_pubkey<S>(bytes: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(bytes.to_bytes()))
}

/// used for serializing base64 to biscuit keypair
pub fn serialize_base64_keypair<S>(bytes: &KeyPair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(bytes.private().to_bytes()))
}
