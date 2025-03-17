use tokio::sync::Mutex;
use tokio::net::TcpListener;
use warp::Filter;
use warp::http;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{BlockData, PendingBlock};

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
  biography:  String,
  public_key: String,
  signature:  String,
}

#[derive(Clone, Serialize)]
struct HealthReply {}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>) {
  let addr: SocketAddr = ([127, 0, 0, 1], 3030).into();

  let health = warp::path("health")
    .and(warp::get())
    .and_then(handle_health);

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

  let routes = health
    .or(feed)
    .or(post)
    .or(user)
    .with(warp::cors()
      .allow_any_origin() // Allow any origin (for development)
      .allow_methods(vec!["GET", "POST"]) // Allow GET and POST requests
      .allow_headers(vec!["Content-Type"])
    );

  match TcpListener::bind(addr).await {
    Ok(listener) => {
      drop(listener);

      println!("Running api on 127.0.0.1:3030");

      warp::serve(routes).run(addr).await;
    }
    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
      println!("API already running on {}, skipping startup.", addr);
    }
    Err(e) => {
      eprintln!("Failed to bind server: {}", e);
    }
  }
}

async fn handle_health() -> Result<impl warp::Reply, warp::Rejection> {
  Ok(warp::reply::json(&HealthReply{}))
}

async fn handle_feed(chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;

  // Step 1: Build a map of public_key -> username from User registrations
  let mut user_map: HashMap<String, String> = HashMap::new();

  for block in &chain.chain {
    if let BlockData::User { username, .. } = &block.data {
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
  let mut chain = chain.lock().await;

  chain.push_mempool(PendingBlock::new(
    BlockData::Post {
      body:   req.clone().body,
      reply:  req.clone().reply,
    },
    req.public_key,
    req.signature,
  )).unwrap_or_else(|e| println!("{}", e));

  Ok(warp::reply::with_status(
    warp::reply(),
    http::StatusCode::NO_CONTENT
  ))
}

async fn handle_user(req: UserRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  println!("test??");

  let mut chain = chain.lock().await;

  chain.push_mempool(PendingBlock::new(
    BlockData::User {
      username:  req.username,
      biography: req.biography,
    },
    req.public_key,
    req.signature,
  )).unwrap_or_else(|e| println!("{}", e));

  Ok(warp::reply::with_status(
    warp::reply(),
    http::StatusCode::NO_CONTENT
  ))
}
