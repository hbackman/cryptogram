use serde::Serialize;
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result};
use crate::blockchain::block::Block;
use crate::blockchain::block::BlockData;

#[derive(Debug, Clone, Serialize)]
pub struct Post {
  pub hash:      String,
  pub author:    User,
  pub body:      String,
  pub reply:     Option<String>,
  pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
  pub display_name: String,
  pub username:     String,
  pub biography:    String,
  pub public_key:   String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostDetail {
  post:     Post,
  replies:  Vec<Post>,
  reply_to: Option<Post>,
}

#[derive(Debug)]
pub struct Index {
  sqlite: Connection,
}

impl Index {
  pub fn new() -> Self {
    let sqlite = Connection::open("chainindex.db").unwrap();

    let _ = sqlite.execute("
      CREATE TABLE IF NOT EXISTS posts (
        hash      TEXT PRIMARY KEY,
        author    TEXT NOT NULL,
        body      TEXT NOT NULL,
        reply     TEXT,
        timestamp INTEGER NOT NULL
      );
    ", []);

    let _ = sqlite.execute("CREATE INDEX IF NOT EXISTS idx_posts_author ON posts (author)", []);
    let _ = sqlite.execute("CREATE INDEX IF NOT EXISTS idx_posts_reply ON posts (reply)", []);

    let _ = sqlite.execute("
      CREATE TABLE IF NOT EXISTS users (
        public_key   TEXT PRIMARY KEY,
        username     TEXT NOT NULL,
        display_name TEXT NOT NULL,
        biography    TEXT
      );
    ", []);

    let _ = sqlite.execute("CREATE INDEX IF NOT EXISTS idx_users_username ON users (username)", []);


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
      BlockData::UserUpdate { .. } => {
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

    if let BlockData::UserUpdate {
      display_name,
      biography,
      ..
    } = block.clone().data {
      self.sqlite.execute("
        UPDATE users
        SET display_name = ?1, biography = ?2
        WHERE public_key = ?3
      ", params![
        display_name,
        biography,
        block.clone().public_key,
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

    let posts = self.sqlite
      .prepare(&query)?
      .query_map(params.as_slice(), |row| {
        Ok(Post {
          author:    User {
            display_name: row.get("display_name")?,
            username:     row.get("username")?,
            biography:    row.get("biography")?,
            public_key:   row.get("public_key")?,
          },
          hash:      row.get("hash")?,
          body:      row.get("body")?,
          reply:     row.get::<_, Option<String>>("reply")?,
          timestamp: row.get::<_, i64>("timestamp")? as u64,
        })
      })?
      .collect::<Result<Vec<Post>, _>>()?;
    Ok(posts)
  }

  /**
   * Retrieve a post by its hash.
   */
  pub fn get_post(&self, hash: &str) -> Result<Option<Post>> {
    self.sqlite.query_row("
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
          display_name: row.get("display_name")?,
          username:     row.get("username")?,
          biography:    row.get("biography")?,
          public_key:   row.get("public_key")?,
        },
        hash:      row.get("hash")?,
        body:      row.get("body")?,
        reply:     row.get::<_, Option<String>>("reply")?,
        timestamp: row.get::<_, i64>("timestamp")? as u64,
      })
    }).optional()
  }

  /**
   * Hydrate a post with full detail.
   */
  pub fn hydrate_post(&self, post: Post) -> Result<PostDetail> {
    let replies = self.get_replies(&post.hash)?;
    let reply_to = post.clone().reply
        .map(|r| self.get_post(&r))
        .transpose()?
        .flatten();

    Ok(PostDetail {
      post,
      reply_to,
      replies,
    })
  }

  pub fn hydrate_feed(&self, feed: Vec<Post>) -> Result<Vec<PostDetail>> {
    feed
      .into_iter()
      .map(|post| self.hydrate_post(post))
      .collect()
  }

  pub fn get_replies(&self, hash: &str) -> Result<Vec<Post>> {
    let posts = self.sqlite
      .prepare("
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
        WHERE posts.reply = ?1
      ")?
      .query_map([hash], |row| {
        Ok(Post {
          author:    User {
            display_name: row.get("display_name")?,
            username:     row.get("username")?,
            biography:    row.get("biography")?,
            public_key:   row.get("public_key")?,
          },
          hash:      row.get("hash")?,
          body:      row.get("body")?,
          reply:     row.get::<_, Option<String>>("reply")?,
          timestamp: row.get::<_, i64>("timestamp")? as u64,
        })
      })?
      .collect::<Result<Vec<Post>, _>>()?;
    Ok(posts)
  }

  /**
   * Retrieve a user by their username.
   */
  pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
    self.sqlite.query_row("
      SELECT
        display_name,
        username,
        biography,
        public_key
      FROM users
      WHERE users.username = ?
    ", [username], |row| {
      Ok(User {
        display_name: row.get("display_name")?,
        username:     row.get("username")?,
        biography:    row.get("biography")?,
        public_key:   row.get("public_key")?,
      })
    }).optional()
  }

  /**
   * Retrieve a user by their username.
   */
  pub fn get_user_by_public_key(&self, public_key: &str) -> Result<Option<User>> {
    self.sqlite.query_row("
      SELECT
        display_name,
        username,
        biography,
        public_key
      FROM users
      WHERE users.public_key = ?
    ", [public_key], |row| {
      Ok(User {
        display_name: row.get("display_name")?,
        username:     row.get("username")?,
        biography:    row.get("biography")?,
        public_key:   row.get("public_key")?,
      })
    }).optional()
  }

  pub fn search_users(&self, username: String) -> Result<Vec<User>> {
    let users = self.sqlite
      .prepare("
        SELECT
          display_name,
          username,
          biography,
          public_key
        FROM users
        WHERE users.username LIKE ?
      ")?
      .query_map([format!("%{}%", username)], |row| {
        Ok(User {
          display_name: row.get("display_name")?,
          username:     row.get("username")?,
          biography:    row.get("biography")?,
          public_key:   row.get("public_key")?,
        })
      })?
      .collect::<Result<Vec<User>, _>>()?;
    Ok(users)
  }

  pub fn has_username(&self, username: &str) -> Result<bool> {
    let res = self.sqlite
      .query_row("SELECT 1 FROM users WHERE username = ?", [&username], |row| row.get::<_, i32>(0))
      .optional()?;
    Ok(res.is_some())
  }

  pub fn has_pubkey(&self, public_key: &str) -> Result<bool> {
    let res = self.sqlite
      .query_row("SELECT 1 FROM users WHERE public_key = ?", [&public_key], |row| row.get::<_, i32>(0))
      .optional()?;
    Ok(res.is_some())
  }
}
