use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
  pub index:     u64,
  pub timestamp: u64,
  pub nonce:     u64,
  pub data:      String,
  pub prev_hash: String,
  pub hash:      String,
}

impl Block {
  pub fn new(index: u64, data: String, previous_hash: String) -> Self {
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
    };

    block.hash = block.hash_block();
    block
  }

  pub fn next(previous: &Block, data: String) -> Self {
    let next_index = previous.index + 1;
    let this_hash = previous.hash.clone();
    Block::new(next_index, data, this_hash)
  }

  pub fn hash_block(&self) -> String {
    let mut hasher = Sha256::new();
    hasher.update(self.index.to_string());
    hasher.update(self.timestamp.to_string());
    hasher.update(self.nonce.to_string());
    hasher.update(&self.data);
    hasher.update(&self.prev_hash);
    format!("{:x}", hasher.finalize())
  }

  /**
   * Mine the block until the hash hits the difficulty.
   */
  pub fn mine_block(&mut self) {
    let target = "00000";

    while !self.hash.starts_with(target) {
      self.nonce += 1;
      self.hash = self.hash_block();
    }

    println!("Block mined! Nonce: {}, Hash: {}", self.nonce, self.hash);
  }

  /**
   * Serialize to json.
   */
  pub fn to_json(&self) -> String {
    serde_json::to_string(&self).unwrap()
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct Blockchain {
  pub chain: Vec<Block>,
}

impl Blockchain {
  pub fn new() -> Self {
    Self {
      chain: vec![Blockchain::genesis()],
    }
  }

  fn genesis() -> Block {
    Block::new(0, "Genesis".to_string(), "0".to_string())
  }

  /**
   * Add a block to the chain.
   */
  pub fn add_block(&mut self, block: Block) {
    if self.validate_block(&block) {
      self.chain.push(block);
    } else {
      println!("Invalid block: {:?}", block);
    }
  }

  fn validate_block(&self, block: &Block) -> bool {
    let target = "00000";

    let last_block = self.latest_block();
    block.prev_hash == last_block.hash && block.hash.starts_with(target)
  }

  /**
   * Retrieve the latest block.
   */
  pub fn latest_block(&self) -> &Block {
    self.chain.last().unwrap()
  }

  /**
   * Serialize to json.
   */
  pub fn to_json(&self, pretty: bool) -> String {
    if pretty {
      serde_json::to_string_pretty(&self.chain).unwrap()
    } else {
      serde_json::to_string(&self.chain).unwrap()
    }
  }

  /**
   * Update the blockchain.
   */
  pub fn update(&mut self, chain: Vec<Block>) {
    if chain.len() > self.chain.len() {
      self.chain = chain;
    }
  }
}
