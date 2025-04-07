use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_qs;
use warp::http::StatusCode;
use warp::Filter;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{BlockData, PendingBlock};
use crate::blockchain::index::PostDetail;
use crate::api::common::{error, reply, no_content, with_chain};

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

  reply(&FeedReply {
    feed
  })
}

/**
 * Handle a new post being made.
 */
async fn handle_post_create(req: PostRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;

  // todo: validate reply hash

  if req.body.len() > 300 {
    return error("Post body cannot exceed 300 characters.", StatusCode::UNPROCESSABLE_ENTITY);
  }

  chain.push_mempool(PendingBlock::new(
    BlockData::Post {
      body:   req.clone().body,
      reply:  req.clone().reply,
    },
    req.public_key,
    req.signature,
  )).unwrap_or_else(|e| println!("{}", e));

  no_content()
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

  reply(&hydrated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::Reply;

    #[tokio::test]
    async fn test_handle_post_create_rejects_long_post() {
      let req = PostRequest {
        body:       "a".repeat(320),
        reply:      None,
        public_key: "dummy_key".to_string(),
        signature:  "dummy_sig".to_string(),
      };

      let chain = Blockchain::new_arc();
      let reply = handle_post_create(req, chain)
        .await
        .unwrap()
        .into_response();

      assert_eq!(reply.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
