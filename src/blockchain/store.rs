use heed::{EnvOpenOptions, Database};
use heed::types::SerdeJson;
use heed::types::U64;
use heed::Env;
use byteorder::NativeEndian;
use crate::blockchain::block::Block;

#[derive(Debug, Clone)]
pub struct Storage {
  pub env: Env,
  pub db: Database<U64<NativeEndian>, SerdeJson<Block>>,
}

impl Storage {
  pub fn new() -> heed::Result<Self> {
    let env = unsafe {
      EnvOpenOptions::new()
        .max_dbs(1)
        .open("blockchain")?
    };

    let db = {
      let mut wtxn = env.write_txn()?;
      let db = env.create_database(&mut wtxn, None)?;
      wtxn.commit()?;
      db
    };

    Ok(Self {
      env, db
    })
  }

  /**
   * Retrieve a block from storage.
   */
  pub fn get_block(&self, index: u64) -> heed::Result<Option<Block>> {
    let rtxn = self.env.read_txn()?;
    let block = self.db.get(&rtxn, &index)?;
    Ok(block)
  }

  /**
   * Persist a block into storage.
   */
  pub fn put_block(&self, block: Block) -> heed::Result<()> {
    let mut wtxn = self.env.write_txn()?;
    self.db.put(&mut wtxn, &block.index, &block)?;
    wtxn.commit()?;
    Ok(())
  }

  /**
   * Retrieve the block on the top of the chain.
   */
  pub fn top_block(&self) -> heed::Result<Block> {
    let rtxn = self.env.read_txn()?;
    let iter = self.db.iter(&rtxn)?;

    iter
      .map(|res| res.map(|(_, block)| block))
      .last()
      .unwrap()
  }

  /**
   * Retrieve the chain height.
   */
  pub fn get_height(&self) -> heed::Result<u64> {
    Ok(self.top_block()?.index)
  }
}
