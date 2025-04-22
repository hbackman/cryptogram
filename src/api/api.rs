use tokio::sync::Mutex;
use tokio::net::TcpListener;
use warp::Filter;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::api::posts::post_routes;
use crate::api::users::user_routes;
use crate::api::links::link_routes;

#[derive(Clone, Serialize)]
struct HealthReply {}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>, addr: String) {
  let addr: SocketAddr = addr.parse().unwrap();

  let health = warp::path("health")
    .and(warp::get())
    .and_then(handle_health);

  let user_routes = user_routes(chain.clone());
  let post_routes = post_routes(chain.clone());
  let link_routes = link_routes();

  let routes = health
    .or(user_routes)
    .or(post_routes)
    .or(link_routes)
    .with(warp::cors()
      .allow_any_origin() // Allow any origin (for development)
      .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
      .allow_headers(vec!["Content-Type"])
    )
    .recover(handle_rejection);

  match TcpListener::bind(addr).await {
    Ok(listener) => {
      drop(listener);

      println!("Running api on {}", addr);

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

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
  if err.is_not_found() {
    Ok(warp::reply::with_status(
      "Not Found",
      warp::http::StatusCode::NOT_FOUND,
    ))
  } else {
    Ok(warp::reply::with_status(
      "Internal Server Error",
      warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    ))
  }
}
