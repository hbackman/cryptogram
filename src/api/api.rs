use tokio::sync::Mutex;
use warp::Filter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{Block, BlockData};

#[derive(Clone, Serialize)]
struct Post {
  author:    String,
  body:      String,
  reply:     Option<String>,
  timestamp: u64,
}

#[derive(Clone, Serialize)]
struct FeedReply {
  feed: Vec<Post>,
}

#[derive(Clone, Deserialize)]
struct PostRequest {
  body:       String,
  reply:      Option<String>,
  public_key: String,
  signature:  String,
}

#[derive(Clone, Deserialize)]
struct UserRequest {
  username:   String,
  public_key: String,
  signature:  String,
}

#[derive(Clone, Serialize)]
struct UserReply {
  hash: String,
}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>) {
  let feed = warp::path("feed")
    .and(warp::get())
    .and(warp::any().map({
      let chain = chain.clone();
      move || chain.clone()
    }))
    .and_then(handle_feed);

  let post = warp::path("post")
    .and(warp::post())
    .and(warp::body::json())
    .and(warp::any().map({
      let chain = chain.clone();
      move || chain.clone()
    }))
    .and_then(handle_post);

  let user = warp::path("user")
      .and(warp::post())
      .and(warp::body::json())
      .and(warp::any().map({
        let chain = chain.clone();
        move || chain.clone()
      }))
      .and_then(handle_user);

  let routes = feed
    .or(post)
    .or(user)
    .with(warp::cors()
      .allow_any_origin() // Allow any origin (for development)
      .allow_methods(vec!["GET", "POST"]) // Allow GET and POST requests
      .allow_headers(vec!["Content-Type"])
    );

  println!("Running api on 127.0.0.1:3030");

  warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_feed(chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;

  // Step 1: Build a map of public_key -> username from User registrations
  let mut user_map: HashMap<String, String> = HashMap::new();

  for block in &chain.chain {
    if let BlockData::User { username } = &block.data {
      user_map.insert(block.public_key.clone(), username.clone());
    }
  }

  // Step 2: Use this map to set `author` in Post blocks
  let posts = chain
    .chain
    .iter()
    .filter_map(|block| {
      if let BlockData::Post { body, reply, .. } = &block.data {
        let author = user_map
          .get(&block.public_key)
          .cloned()
          .unwrap_or_else(|| "Anonymous".to_string()); // Default if no registration

        Some(Post {
          author,
          body:      body.clone(),
          reply:     reply.clone(),
          timestamp: block.timestamp,
        })
      } else {
        None
      }
    })
    .rev()
    .collect();

  Ok(warp::reply::json(&FeedReply{ feed: posts }))
}

async fn handle_post(req: PostRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let data = BlockData::Post {
    body:   req.clone().body,
    reply:  req.clone().reply,
  };

  let mut chain = chain.lock().await;
  let mut block = Block::next(chain.latest_block(), data);

  block.signature = req.clone().signature;
  block.public_key = req.clone().public_key;

  // todo: handle invalid signature
  // todo: notify peers

  println!("handle_post: {}", block.data.to_json());

  block.mine_block();
  chain.add_block(block.clone());

  println!("handle_post: done");

  Ok(warp::reply::with_status(warp::reply(), warp::http::StatusCode::NO_CONTENT))
}

async fn handle_user(req: UserRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let data = BlockData::User {
    username: req.username,
  };

  let mut chain = chain.lock().await;
  let mut block = Block::next(chain.latest_block(), data);

  block.signature = req.signature;
  block.public_key = req.public_key;

  // todo: handle invalid signature
  // todo: notify peers

  println!("handle_user: {}", block.data.to_json());

  block.mine_block();
  chain.add_block(block.clone());

  println!("handle_user: done");

  Ok(warp::reply::json(&UserReply{
    hash: block.hash.to_string(),
  }))
}
