use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
  Chat,
  PeerDiscovery,
  PeerGossip,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
  pub msg_type: MessageType,
  pub sender: String,
  pub payload: String,
}
