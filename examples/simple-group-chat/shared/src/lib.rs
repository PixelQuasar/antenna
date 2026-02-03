use antenna::utils::{Channel, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatClientMsg {
    pub text: String,
}

impl Message for ChatClientMsg {
    fn channel(&self) -> Channel {
        Channel::Reliable
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatServerMsg {
    pub author_id: String,
    pub text: String,
    pub timestamp: u64,
}

impl Message for ChatServerMsg {
    fn channel(&self) -> Channel {
        Channel::Reliable
    }
}
