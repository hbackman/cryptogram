pub mod api;
pub mod p2p;
pub mod blockchain;

use blockchain::chain::Blockchain;
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

  if ! peers.is_empty() {
    println!("Peers: {:?}", peers);
  }

  let chain = Blockchain::new_arc();

  tokio::join!(
    p2p::p2p::start_p2p(chain.clone(), addr),
    api::api::start_api(chain.clone()),
  );
}

fn cli() -> Command {
  Command::new("Cryptogram")
    .version("1.0")
    .about("A decentralized microblogging platform on blockchain.")
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
