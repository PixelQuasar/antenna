use antenna_protocol::PeerID;
use anyhow::Result;

pub trait AntennaSignaling {
    ///
    fn join_mesh(&self, peer_id: PeerID) -> Result<()>;
}
