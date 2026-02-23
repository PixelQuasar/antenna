use crate::model::Channel;
use serde::{Serialize, de::DeserializeOwned};

/// Base web message trait of the object that is sendable between client and server.
pub trait Message: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn channel(&self) -> Channel;

    fn is_rpc(&self) -> bool {
        false
    }
}
