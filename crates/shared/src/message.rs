use serde::{Serialize, de::DeserializeOwned};

pub trait AntennaPayload: Serialize + DeserializeOwned + Clone + 'static {}

impl<T> AntennaPayload for T where T: Serialize + DeserializeOwned + Clone + 'static {}
