use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
  // Misc
  Chat,
  // Peers
  PeerDiscovery,
  PeerGossip,
  // Blockchain
  BlockchainRequest,
  BlockchainReply,
  BlockchainTx,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
  pub msg_type: MessageType,
  pub sender: String,
  pub payload: String,
}
