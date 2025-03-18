use tokio::sync::Mutex;
use tokio::net::TcpListener;
use warp::Filter;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::blockchain::chain::Blockchain;
use crate::api::posts::post_routes;
use crate::api::users::user_routes;

#[derive(Clone, Serialize)]
struct HealthReply {}

/**
 * Start the API.
 */
pub async fn start_api(chain: Arc<Mutex<Blockchain>>) {
  let addr: SocketAddr = ([127, 0, 0, 1], 3030).into();

  let health = warp::path("health")
    .and(warp::get())
    .and_then(handle_health);

  let user_routes = user_routes(chain.clone());
  let post_routes = post_routes(chain.clone());

  let routes = health
    .or(user_routes)
    .or(post_routes)
    .with(warp::cors()
      .allow_any_origin() // Allow any origin (for development)
      .allow_methods(vec!["GET", "POST"]) // Allow GET and POST requests
      .allow_headers(vec!["Content-Type"])
    );

  match TcpListener::bind(addr).await {
    Ok(listener) => {
      drop(listener);

      println!("Running api on 127.0.0.1:3030");

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
