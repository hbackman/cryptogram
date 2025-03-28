use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_qs;
use warp::http;
use warp::Filter;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{BlockData, PendingBlock};
use crate::blockchain::index::PostDetail;

#[derive(Clone, Deserialize)]
pub struct PostRequest {
  body:       String,
  reply:      Option<String>,
  public_key: String,
  signature:  String,
}

#[derive(Debug, Deserialize)]
struct FeedQuery {
  user:   Option<Vec<String>>,
  limit:  Option<usize>,
  offset: Option<usize>,
}

#[derive(Clone, Serialize)]
struct FeedReply {
  feed: Vec<PostDetail>,
}

pub fn post_routes(chain: Arc<Mutex<Blockchain>>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  let feed = warp::path("feed")
    .and(warp::get())
    .and(warp::query::raw())
    .and(with_chain(chain.clone()))
    .and_then(handle_feed);

  let post_create = warp::path("posts")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_chain(chain.clone()))
    .and_then(handle_post_create);

  let post_detail = warp::path!("posts" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_post_detail);

  feed
    .or(post_create)
    .or(post_detail)
}

fn with_chain(
    chain: Arc<Mutex<Blockchain>>,
) -> impl Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || chain.clone())
}

/**
 * Handle the feed endpoint.
 */
async fn handle_feed(query: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let query = serde_qs::from_str::<FeedQuery>(&query)
    .unwrap();

  let chain = chain.lock().await;
  let feed  = chain.index.get_feed(
    query.user.unwrap_or(vec![]),
    query.limit.unwrap_or(32),
    query.offset.unwrap_or(0)
  ).unwrap();

  let feed = chain.index
    .hydrate_feed(feed)
    .map_err(|_| warp::reject::reject())?;

  Ok(warp::reply::json(&FeedReply {
    feed
  }))
}

/**
 * Handle a new post being made.
 */
async fn handle_post_create(req: PostRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;

  // todo: validate reply hash

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

/**
 * Handle a post detail.
 */
async fn handle_post_detail(hash: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;

  let post = chain.index.get_post(&hash)
      .map_err(|_| warp::reject::not_found())?
      .ok_or_else(|| warp::reject::not_found())?;

  let hydrated = chain.index.hydrate_post(post)
      .map_err(|_| warp::reject::not_found())?;

  Ok(warp::reply::json(&hydrated))
}
