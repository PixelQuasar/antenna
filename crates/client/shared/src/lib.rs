use antenna_protocol::PeerID;
use anyhow::Result;

pub trait AntennaClient {
    ///
    fn start(&mut self, peer_id: PeerID) -> Result<String>;

    ///
    fn receive_offer(&mut self, peer_id: PeerID, offer: String) -> Result<String>;

    ///
    fn receive_answer(&mut self, peer_id: PeerID, answer: String) -> Result<()>;

    ///
    fn handshake_queue(&self) -> Vec<PeerID>;
}
