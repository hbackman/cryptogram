use serde::Serialize;
use std::fs;
use crate::blockchain::block::{Block, BlockData};

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
    Block::new(BlockData{
      author: "anonymous".to_string(),
      body:   "genesis".to_string(),
      reply:  None,
    }, 0, "0".to_string())
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
    let target = "0".repeat(block.difficulty());
    let lblock = self.latest_block();

    block.prev_hash == lblock.hash && block.hash.starts_with(&target)
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
    let first0 = self.chain.first().unwrap();
    let first1 =      chain.first().unwrap();

    // If the genesis block is different, or the new chain is longer, then
    // accept the new chain instead.
    if first0.hash != first1.hash || chain.len() > self.chain.len() {
      self.chain = chain;
    }
  }

  /**
   * Save the blockchain to the filesystem.
   */
  pub fn save_to_file(&self, filename: &str) {
    fs::write(filename, self.to_json(true))
      .expect("Failed to save blockchain.");
  }

  /**
   * Load the blockchain from the filesystem.
   */
  pub fn load_from_file(filename: &str) -> Self {
    match fs::read_to_string(filename) {
      Ok(dt) => {
        match serde_json::from_str::<Vec<Block>>(&dt) {
          Ok(chain) => Self{chain},
          Err(_)    => panic!("Failed to parse blockchain json."),
        }
      },
      Err(_) => Self::new(),
    }
  }
}
