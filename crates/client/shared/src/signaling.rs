use serde::{Deserialize, Serialize};

/// WebSocket frame sent from a signaling client to the bundled signaling server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg<'a> {
    /// Enter (or create) a room.
    Join { room_id: &'a str },
    /// Reply to `RequestOffer` with the SDP offer string.
    Offer { room_id: &'a str, offer: &'a str },
    /// Reply to `OfferReceived` with the SDP answer string.
    Answer { room_id: &'a str, answer: &'a str },
    /// Leave the room (also implied by closing the WebSocket).
    Disconnect { room_id: &'a str },
}

/// WebSocket frame sent from the signaling server to a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg<'a> {
    /// "You are the host of this pairing — produce an offer and send it back."
    RequestOffer,
    /// "You are the joiner — produce an answer to this offer."
    OfferReceived { offer: &'a str },
    /// "Your offer was accepted, here is the joiner's answer."
    AnswerReceived { answer: &'a str },
    /// Server-side failure (room full, malformed frame, etc.).
    Error { message: &'a str },
}
