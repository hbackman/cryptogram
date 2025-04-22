pub mod api;
pub mod p2p;
pub mod blockchain;

use blockchain::chain::Blockchain;
use clap::{Arg, ArgMatches, Command};

#[tokio::main]
async fn main() {
  let matches = cli().get_matches();

  // Handle peer argument.
  let peers: Vec<&String> = matches.get_many::<String>("peer")
    .unwrap_or_default()
    .collect();

  if ! peers.is_empty() {
    println!("Peers: {:?}", peers);
  }

  let chain = Blockchain::new_arc();

  tokio::join!(
    p2p::p2p::start_p2p(chain.clone(), get_p2p_addr(matches.clone())),
    api::api::start_api(chain.clone(), get_api_addr(matches.clone())),
  );
}

fn get_p2p_addr(cli: ArgMatches) -> String {
  let port = cli.get_one::<String>("p2p-port").unwrap();

  format!("127.0.0.1:{}", port)
}

fn get_api_addr(cli: ArgMatches) -> String {
  let port = cli.get_one::<String>("api-port").unwrap();

  format!("127.0.0.1:{}", port)
}

fn cli() -> Command {
  Command::new("Cryptogram")
    .version("1.0")
    .about("A decentralized microblogging platform on blockchain.")
    .args([
      Arg::new("p2p-port")
        .long("p2p-port")
        .help("The node port")
        .required(true),
      Arg::new("api-port")
        .long("api-port")
        .help("The API port")
        .default_value("3030")
        .required(false),
      Arg::new("peer")
        .long("peer")
        .help("A peer to connect to")
        .num_args(1..),
    ])
}
