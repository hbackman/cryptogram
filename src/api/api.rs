use tokio::sync::Mutex;
use warp::Filter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{Block, BlockData};

#[derive(Serialize)]
struct Post {
  author:    String,
  body:      String,
  reply:     Option<String>,
  timestamp: u64,
}

#[derive(Serialize)]
struct FeedReply {
  feed: Vec<Post>,
}

#[derive(Deserialize)]
struct PostRequest {
  author: String,
  body:   String,
  reply:  Option<String>,
}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>) {
  let cors = warp::cors()
    .allow_any_origin() // Allow any origin (for development)
    .allow_methods(vec!["GET", "POST"]) // Allow GET and POST requests
    .allow_headers(vec!["Content-Type"]);

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

  let routes = feed.or(post).with(cors.clone());

  println!("Running api on 127.0.0.1:3030");

  warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_feed(chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;

  let posts = chain
    .chain
    .iter()
    .map(|block| Post{
      author:    block.data.clone().author,
      body:      block.data.clone().body,
      reply:     block.data.clone().reply,
      timestamp: block.timestamp,
    })
    .rev()
    .collect();

  Ok(warp::reply::json(&FeedReply{ feed: posts }))
}

async fn handle_post(req: PostRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;
  let mut block = Block::next(chain.latest_block(), BlockData{
    author: req.author,
    body:   req.body,
    reply:  req.reply,
  });

  block.mine_block();
  chain.add_block(block.clone());

  println!("mined new block");

  // todo: notify peers

  Ok(warp::reply::json(&FeedReply{
    feed: vec![Post{
      author:    block.data.clone().author,
      body:      block.data.clone().body,
      reply:     block.data.clone().reply,
      timestamp: block.timestamp,
    }]
  }))
}
