use serde::{Serialize, Deserialize};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::blockchain::sign::Keypair;
use crate::blockchain::sign::ValidationError;
use crate::blockchain::sign::validate_signature;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
  pub index:      u64,
  pub timestamp:  u64,
  pub nonce:      u64,
  pub data:       BlockData,
  pub prev_hash:  String,
  pub hash:       String,
  pub public_key: String,
  pub signature:  String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingBlock {
  pub timestamp:  u64,
  pub data:       BlockData,
  pub public_key: String,
  pub signature:  String,
}

impl PendingBlock {
  pub fn new(data: BlockData, public_key: String, signature: String) -> Self {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards")
      .as_secs();

    PendingBlock {
      timestamp,
      data,
      public_key,
      signature,
    }
  }

  /**
   * Validate the block signature.
   */
  pub fn validate_signature(&self) -> Result<(), ValidationError> {
    validate_signature(
      &self.public_key,
      &self.signature,
      &self.data.to_json_for_signing()
    )
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BlockData {
  Genesis {
    //
  },
  User {
    username: String,
  },
  Post {
    body:  String,
    reply: Option<String>,
  }
}

impl BlockData {
  pub fn to_json(&self) -> String {
    serde_json::to_string(&self).unwrap()
  }

  pub fn to_json_for_signing(&self) -> String {
    let json = serde_json::to_string(self).unwrap();
    let mut value = serde_json::from_str(&json).unwrap();

    if let Value::Object(ref mut map) = value {
      map.remove("type");
    }

    serde_json::to_string(&value).unwrap()
  }
}

impl Block {
  pub fn new(data: BlockData, index: u64, previous_hash: String) -> Self {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards")
      .as_secs();

    let mut block = Block {
      index,
      timestamp,
      nonce: 0,
      data,
      prev_hash: previous_hash.clone(),
      hash: String::new(), // placeholder
      public_key: "".to_string(),
      signature:  "".to_string(),
    };

    block.hash = block.hash_block();
    block
  }

  pub fn next(previous: &Block, data: BlockData) -> Self {
    let next_index = previous.index + 1;
    let this_hash = previous.hash.clone();
    Block::new(data, next_index, this_hash)
  }

  pub fn hash_block(&self) -> String {
    let mut hasher = Sha256::new();
    hasher.update(self.index.to_string());
    hasher.update(self.timestamp.to_string());
    hasher.update(self.nonce.to_string());
    hasher.update(self.data.to_json());
    hasher.update(self.signature.to_string());
    hasher.update(self.public_key.to_string());
    hasher.update(&self.prev_hash);
    format!("{:x}", hasher.finalize())
  }

  /**
   * Mine the block until the hash hits the difficulty.
   */
  pub fn mine_block(&mut self) {
    let target = "0".repeat(self.difficulty());

    while !self.hash.starts_with(&target) {
      self.nonce += 1;
      self.hash = self.hash_block();
    }

    println!("Block mined! Nonce: {}, Hash: {}", self.nonce, self.hash);
  }

  /**
   * Sign the block.
   */
  pub fn sign_block(&mut self, keypair: Keypair) {
    self.signature = keypair.sign_message(&self.data.to_json());
    self.public_key = keypair.get_public_key()
  }

  /**
   * Validate the block signature.
   */
  pub fn validate_signature(&self) -> Result<(), ValidationError> {
    validate_signature(
      &self.public_key,
      &self.signature,
      &self.data.to_json_for_signing()
    )
  }

  /**
   * The block difficulty.
   */
  pub fn difficulty(&self) -> usize {
    3
  }

  /**
   * Serialize to json.
   */
  pub fn to_json(&self) -> String {
    serde_json::to_string(&self).unwrap()
  }
}
