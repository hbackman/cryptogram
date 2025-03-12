use tokio::sync::Mutex;
use warp::Filter;
use serde::Serialize;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;

#[derive(Serialize)]
struct PostsReply {
  posts: Vec<Post>,
}

#[derive(Serialize)]
struct Post {
  author:    String,
  body:      String,
  timestamp: u64,
}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>) {
  let posts = warp::path("posts")
    .and(warp::get())
    .and(warp::any().map(move || chain.clone()))
    .and_then(handle_posts);

  let routes = posts;

  println!("Running API on http://127.0.0.1:3030");

  warp::serve(routes)
    .run(([127, 0, 0, 1], 3030))
    .await;
}

async fn handle_posts(chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;

  let posts = chain
    .chain
    .iter()
    .map(|block| Post{
      author:    block.data.clone().author,
      body:      block.data.clone().body,
      timestamp: block.timestamp,
    })
    .collect();

  Ok(warp::reply::json(&PostsReply{ posts }))
}
