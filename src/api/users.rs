use tokio::sync::Mutex;
use serde::Deserialize;
use warp::http;
use warp::Filter;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{BlockData, PendingBlock};

#[derive(Clone, Deserialize)]
pub struct UserRequest {
  display_name: String,
  username:     String,
  biography:    String,
  public_key:   String,
  signature:    String,
}

pub fn user_routes(chain: Arc<Mutex<Blockchain>>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  let create_user = warp::path("users")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_post);

  let show_user = warp::path!("users" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_get);

  let search_user = warp::path!("users" / "search" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_search);

  create_user.or(show_user).or(search_user)
}

fn with_chain(
    chain: Arc<Mutex<Blockchain>>,
) -> impl Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || chain.clone())
}

/**
 * Handle user registration.
 */
async fn handle_user_post(req: UserRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;

  chain.push_mempool(PendingBlock::new(
    BlockData::User {
      display_name: req.display_name,
      username:     req.username,
      biography:    req.biography,
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
 * Handle user details.
 */
async fn handle_user_get(username: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;
  let user = chain.index.get_user(&username);

  user
    .map(|user| warp::reply::json(&user))
    .map_err(|_| warp::reject::not_found())
}

/**
 * Handle user searches.
 */
async fn handle_user_search(search: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;
  let users = chain.index.search_users(search);

  users
    .map(|users| warp::reply::json(&users))
    .map_err(|_| warp::reject::not_found())
}
