use serde::Serialize;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub index:     u64,
    pub timestamp: u64,
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
            data,
            prev_hash: previous_hash.clone(),
            hash: String::new(), // placeholder
        };

        block.hash = block.hash_block();
        block
    }

    pub fn genesis() -> Self {
        Block::new(0, "Genesis".to_string(), "0".to_string())
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
        hasher.update(&self.data);
        hasher.update(&self.prev_hash);
        format!("{:x}", hasher.finalize())
    }
}
