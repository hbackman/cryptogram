mod p2p;
mod api;
mod block;

use std::env;
// use api::start_api;

#[tokio::main]
async fn main() {
    // start_api().await;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <port>", args[0]);
        return;
    }

    let port = &args[1];
    let addr = format!("127.0.0.1:{}", port);

    p2p::start_p2p_server(addr).await;
}
