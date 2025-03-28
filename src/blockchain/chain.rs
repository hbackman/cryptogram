use std::collections::HashSet;
use crate::blockchain::store::Store;
use crate::blockchain::index::Index;
use crate::blockchain::block::{Block, BlockData, PendingBlock};

#[derive(Debug)]
pub struct Blockchain {
  pub mpool: Vec<PendingBlock>,
  pub store: Store,
  pub index: Index,
}

impl Blockchain {
  pub fn new() -> Self {
    let mut chain = Self {
      mpool: vec![],
      store: Store::new().unwrap(),
      index: Index::new(),
    };

    chain.add_block(Blockchain::genesis())
      .unwrap_or_else(|e| println!("{}", e));

    // Catch the index up.
    for block in chain.chain_iter() {
      let _ = chain.index.add_block(block);
    }

    chain
  }

  fn genesis() -> Block {
    Block::new(BlockData::Genesis {}, 0, "0".to_string())
  }

  /**
   * Retrieve the size of the chain.
   */
  pub fn len(&self) -> usize {
    self.store.get_height().unwrap() as usize
  }

  /**
   * Retrieve a block at the given index.
   */
  pub fn at(&self, index: usize) -> Option<Block> {
    self.store
      .get_block(index as u64)
      .unwrap()
  }

  /**
   * Add a block to the chain.
   */
  pub fn add_block(&mut self, block: Block) -> Result<(), String> {
    if block.index > 0 {
      block.validate_signature().map_err(|e| e.to_string())?;

      self.validate_hash(&block)?;
      self.validate_user(&block)?;
    }

    let _ = self.store.put_block(block.clone());
    let _ = self.index.add_block(block);

    Ok(())
  }

  /**
   * Add a block to the memory pool.
   */
  pub fn push_mempool(&mut self, block: PendingBlock) -> Result<(), String> {
    block.validate_signature().map_err(|e| e.to_string())?;
    block.validate_size()?;

    self.mpool.push(block);

    Ok(())
  }

  /**
   * Validate that the block contains the previous hash and that the difficulty
   * was met during block mining.
   */
  fn validate_hash(&self, block: &Block) -> Result<(), String> {
    let target = "0".repeat(block.difficulty());
    let lblock = self.top_block();

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

    for i in 0..=self.store.get_height().unwrap() {
      let block = self.store.get_block(i)
        .unwrap()
        .unwrap();

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
  pub fn top_block(&self) -> Block {
    self.store.top_block().unwrap()
  }

  /**
   * Print the chain to stdout.
   */
  pub fn print_chain(&self) {
    println!("==================================================================================");
    for block in self.chain_iter() {
      let json = serde_json::to_string_pretty(&block)
        .unwrap();

      println!("{}", json);
      println!("==================================================================================");
    }
  }

  pub fn chain_iter(&self) -> impl Iterator<Item = Block> + '_ {
    let height = self.store
      .get_height()
      .unwrap();

    (0..=height).map(move |i| {
      self.store.get_block(i)
        .unwrap()
        .unwrap()
    })
  }
}
