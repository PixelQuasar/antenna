mod dispatcher;
mod ice;
mod signaling;
mod storage;

pub use dispatcher::{CallbackId, Dispatcher, RtcEvent};
pub use ice::IceServerConfig;
pub use signaling::{ClientMsg, ServerMsg};
pub use storage::IdentityStorage;

pub const EXECUTE_FUEL: u64 = 1024;
pub const STORAGE_IDENTITY_KEY: &str = "antenna_identity";
