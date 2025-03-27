use rusqlite::{params, Connection, Result};
use crate::blockchain::block::Block;
use crate::blockchain::block::BlockData;
use crate::blockchain::chain::{Post, User};

#[derive(Debug)]
pub struct Index {
  sqlite: Connection,
}

impl Index {
  pub fn new() -> Self {
    let sqlite = Connection::open("index.db").unwrap();

    let _ = sqlite.execute("
      CREATE TABLE IF NOT EXISTS posts (
        hash      TEXT PRIMARY KEY,
        author    TEXT NOT NULL,
        body      TEXT NOT NULL,
        reply     TEXT,
        timestamp INTEGER NOT NULL
      );
    ", []);

    let _ = sqlite.execute("
      CREATE TABLE IF NOT EXISTS users (
        public_key   TEXT PRIMARY KEY,
        username     TEXT NOT NULL,
        display_name TEXT NOT NULL,
        biography    TEXT
      );
    ", []);

    Self { sqlite }
  }

  /**
   * Add a block to the index.
   */
  pub fn add_block(&self, block: Block) -> Result<(), rusqlite::Error> {
    match block.clone().data {
      BlockData::Post {..} => {
        self.index_post(block)?;
      },
      BlockData::User {..} => {
        self.index_user(block)?;
      },
      _ => {}
    }
    Ok(())
  }

  fn index_post(&self, block: Block) -> Result<(), rusqlite::Error> {
    if let BlockData::Post { body, reply, .. } = block.clone().data {
      self.sqlite.execute("
        INSERT OR IGNORE INTO posts
        (hash, author, body, reply, timestamp) VALUES
        (?1, ?2, ?3, ?4, ?5)
      ", params![
        block.clone().hash,
        block.clone().public_key,
        body,
        reply,
        block.clone().timestamp,
      ])?;
    }
    Ok(())
  }

  fn index_user(&self, block: Block) -> Result<(), rusqlite::Error> {
    if let BlockData::User {
      display_name,
      username,
      biography,
      ..
    } = block.clone().data {
      self.sqlite.execute("
        INSERT OR IGNORE INTO users
        (public_key, username, display_name, biography) VALUES
        (?1, ?2, ?3, ?4)
      ", params![
        block.clone().public_key,
        username,
        display_name,
        biography,
      ])?;
    }
    Ok(())
  }

  /**
   * Retrieve a feed for a set of users.
   */
  pub fn get_feed(&self, users: Vec<String>, limit: usize, offset: usize) -> Result<Vec<Post>> {
    let placeholders = users
        .iter()
        .map(|_| "?".to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!("
      SELECT
        posts.hash,
        posts.body,
        posts.reply,
        posts.timestamp,
        users.display_name,
        users.username,
        users.biography,
        users.public_key
      FROM posts
      JOIN users ON users.public_key = posts.author
      WHERE users.username IN ({})
      LIMIT ?
      OFFSET ?
    ", placeholders);

    let mut params: Vec<&dyn rusqlite::ToSql> = users
      .iter()
      .map(|u| u as &dyn rusqlite::ToSql)
      .collect();

    params.push(&limit);
    params.push(&offset);

    Ok(
      self.sqlite
        .prepare(&query)?
        .query_map(params.as_slice(), |row| {
          Ok(Post {
            author:    User {
              display_name: row.get(4)?,
              username:     row.get(5)?,
              biography:    row.get(6)?,
              public_key:   row.get(7)?,
            },
            hash:      row.get(0)?,
            body:      row.get(1)?,
            reply:     row.get::<_, Option<String>>(2)?,
            timestamp: row.get::<_, i64>(3)? as u64,
          })
        })?
        .collect::<Result<Vec<Post>, _>>()?
    )
  }

  /**
   * Retrieve a post by its hash.
   */
  pub fn get_post(&self, hash: String) -> Result<Post> {
    self.sqlite
      .query_row("
        SELECT
          posts.hash,
          posts.body,
          posts.reply,
          posts.timestamp,
          users.display_name,
          users.username,
          users.biography,
          users.public_key
        FROM posts
        JOIN users ON users.public_key = posts.author
        WHERE posts.hash = ?1
      ", [hash], |row| {
        Ok(Post {
          author:    User {
            display_name: row.get(4)?,
            username:     row.get(5)?,
            biography:    row.get(6)?,
            public_key:   row.get(7)?,
          },
          hash:      row.get(0)?,
          body:      row.get(1)?,
          reply:     row.get::<_, Option<String>>(2)?,
          timestamp: row.get::<_, i64>(3)? as u64,
        })
      })
  }
}
