use serde::{Serialize, Deserialize};
use crate::blockchain::block::Block;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MessageData {
  // Peers
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
