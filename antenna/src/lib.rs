pub use antenna_core::model::PeerId;

pub mod model {
    pub use antenna_core::model::*;
}

#[cfg(feature = "server")]
pub mod server {
    pub use antenna_server::*;
}

#[cfg(feature = "client")]
pub mod client {
    pub use antenna_wasm_gen::*;
}
