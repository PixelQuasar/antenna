use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg<'a> {
    Join { room_id: &'a str },
    Offer { room_id: &'a str, offer: &'a str },
    Answer { room_id: &'a str, answer: &'a str },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg<'a> {
    RequestOffer,
    OfferReceived { offer: &'a str },
    AnswerReceived { answer: &'a str },
    Error { message: &'a str },
}
