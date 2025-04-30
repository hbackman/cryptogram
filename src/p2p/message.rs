use serde::{Serialize, Deserialize};
use crate::blockchain::block::Block;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MessageData {
  // Handshake
  Handshake {
    version: String,
    peer_id: String,
  },
  // Peers
  PeerDiscovery {},
  PeerGossip {
    peers: Vec<String>
  },
  BlockchainTx {
    block: Block,
  },
  BlockRequest { index: usize },
  BlockResponse { block: Block },
  // Misc
  Chat {
    message: String,
  },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message
{
    pub sender: String,
    pub payload: MessageData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Handshake
{
    pub version: String,
    pub peer_id: String,
}
