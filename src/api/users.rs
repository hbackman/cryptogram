use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use warp::http;
use warp::Filter;
use std::sync::Arc;
use http::StatusCode;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{BlockData, PendingBlock};
use crate::api::common::{error, reply, with_chain};

#[derive(Clone, Deserialize)]
pub struct UserCreateRequest {
  display_name: String,
  username:     String,
  biography:    String,
  public_key:   String,
  signature:    String,
}

#[derive(Clone, Deserialize)]
pub struct UserUpdateRequest {
  display_name: String,
  biography:    String,
  public_key:   String,
  signature:    String,
}

#[derive(Clone, Serialize)]
pub struct ErrorReply {
  message: String,
}

pub fn user_routes(chain: Arc<Mutex<Blockchain>>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  let create_user = warp::path("users")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_create);

  let update_user = warp::path!("users" / String)
    .and(warp::put())
    .and(warp::body::json())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_update);

  let user_by_pkey = warp::path!("users" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_by_pkey);

  let user_by_name = warp::path!("users" / "h" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_by_name);

  let user_search = warp::path!("users" / "s" / String)
    .and(warp::get())
    .and(with_chain(chain.clone()))
    .and_then(handle_user_search);

  create_user
    .or(update_user)
    .or(user_search)
    .or(user_by_pkey)
    .or(user_by_name)
}

/**
 * Handle user registration.
 */
async fn handle_user_create(req: UserCreateRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;

  if chain.index.has_username(&req.username).unwrap() {
    return error("Username is already taken.", StatusCode::UNPROCESSABLE_ENTITY);
  }

  if chain.index.has_pubkey(&req.public_key).unwrap() {
    return error("Public key is already taken.", StatusCode::UNPROCESSABLE_ENTITY);
  }

  chain.push_mempool(PendingBlock::new(
    BlockData::User {
      display_name: req.display_name,
      username:     req.username,
      biography:    req.biography,
    },
    req.public_key,
    req.signature,
  )).unwrap_or_else(|e| println!("{}", e));

  Ok(
    warp::reply::with_status(
      warp::reply::json(&{}),
      StatusCode::NO_CONTENT
    )
  )
}

async fn handle_user_update(public_key: String, req: UserUpdateRequest, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let mut chain = chain.lock().await;

  if public_key != req.public_key {
    return error("Public key does not match.", StatusCode::UNAUTHORIZED);
  }

  chain.push_mempool(PendingBlock::new(
    BlockData::UserUpdate {
      display_name: req.display_name,
      biography:    req.biography,
    },
    public_key,
    req.signature,
  )).unwrap_or_else(|e| println!("{}", e));

  Ok(
    warp::reply::with_status(
      warp::reply::json(&{}),
      StatusCode::NO_CONTENT
    )
  )
}

/**
 * Handle user details.
 */
async fn handle_user_by_name(username: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;
  let user = chain.index.get_user_by_username(&username);

  match user {
    Ok(Some(user)) => reply(&user),
    Ok(None)       => error("User could not be found.", StatusCode::NOT_FOUND),
    Err(_)         => error("User could not be found.", StatusCode::NOT_FOUND),
  }
}

async fn handle_user_by_pkey(public_key: String, chain: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
  let chain = chain.lock().await;
  let user = chain.index.get_user_by_public_key(&public_key);

  match user {
    Ok(Some(user)) => reply(&user),
    Ok(None)       => error("User could not be found.", StatusCode::NOT_FOUND),
    Err(_)         => error("User could not be found.", StatusCode::NOT_FOUND),
  }
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

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::Reply;
    use warp::hyper::body::to_bytes;
    use serde_json::Value;

    #[tokio::test]
    async fn test_handle_user_post_rejects_existing_username() {
      let req = UserCreateRequest {
        display_name: "Hampus Backman".to_string(),
        username:     "hbackman".to_string(),
        biography:    "lorem ipsum dolor sit amet".to_string(),
        public_key:   "dummy_key".to_string(),
        signature:    "dummy_sig".to_string(),
      };

      // todo: this is using the real blockchain. it must use a test one
      let chain = Blockchain::new_arc();
      let reply = handle_user_create(req, chain)
        .await
        .unwrap()
        .into_response();

      assert_eq!(reply.status(), StatusCode::UNPROCESSABLE_ENTITY);

      let body = to_bytes(reply.into_body()).await.unwrap();
      let json: Value = serde_json::from_slice(&body).unwrap();

      assert_eq!(json["message"], "Username is already taken.");
    }
}
