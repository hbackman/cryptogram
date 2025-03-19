use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use warp::http;
use warp::Filter;
use std::sync::Arc;
use crate::blockchain::chain::{Blockchain, Post};
use crate::blockchain::block::{BlockData, PendingBlock};

#[derive(Clone, Deserialize)]
pub struct PostRequest {
  body:       String,
  reply:      Option<String>,
  public_key: String,
  signature:  String,
}

#[derive(Debug, Deserialize)]
struct FeedQuery {
  user: Option<String>,
}

#[derive(Clone, Serialize)]
struct FeedReply {
  feed: Vec<Post>,
}

pub fn post_routes(chain: Arc<Mutex<Blockchain>>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  let feed = warp::path("feed")
    .and(warp::get())
    .and(warp::query::<FeedQuery>())
    .and(with_chain(chain.clone()))
    .and_then(handle_feed);

  let post = warp::path("post")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_chain(chain.clone()))
    .and_then(handle_post);

  feed.or(post)
}

fn with_chain(
    chain: Arc<Mutex<Blockchain>>,
) -> impl Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || chain.clone())
}

/**
 * Handle the feed endpoint.
 */
async fn handle_feed(query: FeedQuery, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;
  let posts = chain.get_posts();

  if let Some(user) = query.user {
    let posts: Vec<Post> = posts
        .into_iter()
        .filter(|post| post.author.username == user)
        .collect();

    Ok(warp::reply::json(&FeedReply{
      feed: posts,
    }))
  } else {
    Ok(warp::reply::json(&FeedReply{
      feed: posts,
    }))
  }
}

/**
 * Handle a new post being made.
 */
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
