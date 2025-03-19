use serde::Serialize;
use std::fs;
use std::collections::HashSet;
use std::collections::HashMap;
use crate::blockchain::block::{Block, BlockData, PendingBlock};

#[derive(Debug, Clone, Serialize)]
pub struct Post {
  pub author:    User,
  pub body:      String,
  pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
  pub display_name: String,
  pub username:     String,
  pub public_key:   String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Blockchain {
  pub chain: Vec<Block>,
  pub mpool: Vec<PendingBlock>,
}

impl Blockchain {
  pub fn new() -> Self {
    Self {
      chain: vec![Blockchain::genesis()],
      mpool: vec![],
    }
  }

  fn genesis() -> Block {
    Block::new(BlockData::Genesis {}, 0, "0".to_string())
  }

  /**
   * Add a block to the chain.
   */
  pub fn add_block(&mut self, block: Block) -> Result<(), String> {
    block.validate_signature().map_err(|e| e.to_string())?;

    self.validate_hash(&block)?;
    self.validate_user(&block)?;

    self.chain.push(block);

    Ok(())
  }

  pub fn push_mempool(&mut self, block: PendingBlock) -> Result<(), String> {
    block.validate_signature().map_err(|e| e.to_string())?;

    self.mpool.push(block);

    Ok(())
  }

  /**
   * Validate that the block contains the previous hash and that the difficulty
   * was met during block mining.
   */
  fn validate_hash(&self, block: &Block) -> Result<(), String> {
    let target = "0".repeat(block.difficulty());
    let lblock = self.latest_block();

    if block.prev_hash != lblock.hash {
      return Err("Block hash did not match previous hash.".to_string());
    }

    if ! block.hash.starts_with(&target) {
      return Err("Block hash did not meet difficulty.".to_string());
    }

    Ok(())
  }

  /**
   * Validate that the user is registered before they are allowed to create a
   * new block. This only applies for `Post` block data.
   */
  fn validate_user(&self, block: &Block) -> Result<(), String> {
    let mut user_names: HashSet<String> = HashSet::new();
    let mut user_pkeys: HashSet<String> = HashSet::new();

    for block in &self.chain {
      if let BlockData::User { username, .. } = &block.data {
        user_names.insert(username.clone());
        user_pkeys.insert(block.public_key.clone());
      }
    }

    // Validate user registration.
    if let BlockData::User { username, .. } = block.data.clone() {
      if user_names.contains(&username) {
        return Err(format!("Username '{}' is already taken.", username));
      }

      if user_pkeys.contains(&block.public_key) {
        return Err(format!("Public key '{}' is already registered.", block.public_key));
      }

      return Ok(());
    }

    // Validate post.
    if let BlockData::Post {..} = block.data.clone() {
      if !user_pkeys.contains(&block.public_key) {
        return Err(format!("Public key '{}' is not registered.", block.public_key));
      }

      return Ok(());
    }

    Ok(())
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
          Ok(chain) => Self{
            chain,
            mpool: vec![],
          },
          Err(_)    => panic!("Failed to parse blockchain json."),
        }
      },
      Err(_) => Self::new(),
    }
  }

  /**
   * Retrieve users from the user registration blocks. This returns a hash map
   * with <public_key, username>.
   */
  pub fn get_users(&self) -> HashMap<String, User> {
    let mut map: HashMap<String, User> = HashMap::new();

    for block in &self.chain {
      if let BlockData::User {
        username,
        display_name,
        ..
      } = &block.data {
        map.insert(block.public_key.clone(), User{
          username:     username.to_string(),
          display_name: display_name.to_string(),
          public_key:   block.clone().public_key,
        });
      }
    }

    map
  }

  /**
   * Retrieve posts from the blockchain.
   */
  pub fn get_posts(&self) -> Vec<Post> {
    let mut posts: Vec<Post> = vec![];
    let users = self.get_users();

    for block in &self.chain {
      if let BlockData::Post { body, .. } = &block.data {
        let author = users
          .get(&block.public_key)
          .cloned()
          .unwrap();

        posts.push(Post{
          author,
          body:      body.to_string(),
          timestamp: block.timestamp,
        });
      }
    }

    posts.reverse();
    posts
  }
}
