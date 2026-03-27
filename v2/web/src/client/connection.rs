use antenna_protocol::{Host, Joiner};

use crate::{driver::Driver, utils::IceServerConfig};

pub enum Connection {
    Connecting { ice_servers: Vec<IceServerConfig> },
    Host(Driver<Host>),
    Joiner(Driver<Joiner>),
}
