use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_qs;
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
  user:   Option<Vec<String>>,
  limit:  Option<usize>,
  offset: Option<usize>,
}

#[derive(Clone, Serialize)]
struct FeedReply {
  feed: Vec<Post>,
}

#[derive(Clone, Serialize)]
struct PostReply {
  post:     Post,
  replies:  Vec<Post>,
  reply_to: Option<Post>,
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

  let feed = chain.index.get_feed(
    query.user.unwrap_or(vec![]),
    query.limit.unwrap_or(32),
    query.offset.unwrap_or(0)
  ).unwrap();

  Ok(warp::reply::json(&FeedReply{
    feed
  }))

//   if let Some(user) = query.user {
//     let posts = posts
//       .clone()
//       .into_iter()
//       .filter(|post| user.contains(&post.author.username))
//       .skip(query.offset.unwrap_or(0))
//       .take(query.limit.unwrap_or(32))
//       .map(|post| hydrate_post(post.clone(), posts.clone()))
//       .collect();
//
//     Ok(warp::reply::json(&FeedReply{
//       feed: posts,
//     }))
//   } else {
//     let posts = posts
//       .clone()
//       .into_iter()
//       .skip(query.offset.unwrap_or(0))
//       .take(query.limit.unwrap_or(32))
//       .map(|post| hydrate_post(post.clone(), posts.clone()))
//       .collect();
//
//     Ok(warp::reply::json(&FeedReply{
//       feed: posts,
//     }))
//   }
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

  match chain.index.get_post(hash) {
    Ok(post) => {
      Ok(warp::reply::json(&PostReply{
        post,
        reply_to: None,
        replies: vec![],
      }))
    },
    Err(_) => {
      Err(warp::reject::not_found())
    }
  }



//  let posts = chain.get_posts();
//
//  // todo: improve
//  let post = posts
//    .iter()
//    .find(|post| post.hash == hash);
//
//  match post {
//    Some(post) => {
//      Ok(warp::reply::json(
//        &hydrate_post(post.clone(), posts.clone())
//      ))
//    },
//    None => {
//      Err(warp::reject::not_found())
//    }
//  }
}

fn hydrate_post(post: Post, posts: Vec<Post>) -> PostReply {
  let replies = posts
    .iter()
    .filter(|p| p.reply.as_ref().map(|r| r == &post.hash).unwrap_or(false))
    .cloned()
    .collect();

  let reply_to = posts
    .iter()
    .find(|p| post.reply.as_ref().map(|r| r == &p.hash).unwrap_or(false))
    .cloned();

  PostReply {
    post,
    replies,
    reply_to,
  }
}
