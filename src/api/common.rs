use serde::Serialize;
use warp::Filter;
use warp::http;
use warp::reply::{Json, WithStatus};
use http::StatusCode;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::blockchain::chain::Blockchain;

#[derive(Clone, Serialize)]
pub struct ErrorReply {
  message: String,
}

pub fn with_chain(
  chain: Arc<Mutex<Blockchain>>,
) -> impl Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
  warp::any().map(move || chain.clone())
}

/**
 * Create an error response with the given message and status code.
 */
pub fn error(message: &str, status: StatusCode) -> Result<WithStatus<Json>, warp::Rejection> {
  Ok(warp::reply::with_status(
    warp::reply::json(&ErrorReply{
      message: message.to_string(),
    }),
    status
  ))
}

/**
 * Create a successful response with the given data.
 */
pub fn reply<T>(response: &T) -> Result<WithStatus<Json>, warp::Rejection>
where T: Serialize
{
  Ok(warp::reply::with_status(
    warp::reply::json(response),
    StatusCode::OK
  ))
}

pub fn no_content() -> Result<WithStatus<Json>, warp::Rejection> {
  Ok(warp::reply::with_status(
    warp::reply::json(&{}),
    StatusCode::NO_CONTENT
  ))
}
