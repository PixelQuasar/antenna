pub mod utils {
    pub use antenna_core::*;
}

#[cfg(feature = "server")]
pub mod server {
    pub use antenna_codegen::{antenna_handlers, antenna_room};
    pub use antenna_server::*;
}

#[cfg(feature = "client")]
pub mod client {
    pub use antenna_codegen::antenna_client;
    pub use antenna_wasm_gen::*;
}
