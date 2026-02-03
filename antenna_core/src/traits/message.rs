use crate::model::Channel;
use serde::{Serialize, de::DeserializeOwned};

pub trait Message: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn channel(&self) -> Channel;

    fn is_rpc(&self) -> bool {
        false
    }
}
