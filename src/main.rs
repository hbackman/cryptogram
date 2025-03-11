mod api;
mod p2p;
mod blockchain;
use clap::{Command, Arg};

#[tokio::main]
async fn main() {
  let matches = cli().get_matches();

  // Handle port argument.
  let port = matches.get_one::<String>("port").unwrap();
  let addr = format!("127.0.0.1:{}", port);

  // Handle peer argument.
  let peers: Vec<&String> = matches.get_many::<String>("peer")
    .unwrap_or_default()
    .collect();

  if peers.is_empty() {
      println!("No peers provided.");
  } else {
      println!("Peers: {:?}", peers);
  }

  p2p::node::start_p2p_node(addr).await;
}

fn cli() -> Command {
  Command::new("p2p")
    .version("1.0")
    .about("a p2p test app")
    .args([
      Arg::new("port")
        .long("port")
        .help("The node port")
        .required(true),
      Arg::new("peer")
        .long("peer")
        .help("A peer to connect to")
        .num_args(1..),
    ])
}
