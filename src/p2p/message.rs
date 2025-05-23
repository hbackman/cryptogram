use serde::{Serialize, Deserialize};
use crate::blockchain::block::Block;
use crate::p2p::peer::PeerInfo;

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
    peers: Vec<PeerInfo>,
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
  pub payload:  MessageData,
  pub receiver: Option<String>,
  pub sender:   Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Handshake
{
    pub version: String,
    pub peer_id: String,
    pub addr:    String,
}
