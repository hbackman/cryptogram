pub mod api;
pub mod p2p;
pub mod blockchain;

use toml;
use std::fs;
use std::error::Error;
use serde::Deserialize;
use clap::{Arg, ArgMatches, Command};
use blockchain::chain::Blockchain;
use p2p::p2p::start_p2p;
use api::api::start_api;

#[derive(Debug, Deserialize)]
struct Config {
  peers: Vec<String>,
}

#[tokio::main]
async fn main() {
  let matches = cli().get_matches();
  let config = get_config().unwrap();
  let chain = Blockchain::new_arc();

  tokio::join!(
    start_p2p(chain.clone(), get_p2p_addr(matches.clone()), config.peers),
    start_api(chain.clone(), get_api_addr(matches.clone())),
  );
}

fn get_p2p_addr(cli: ArgMatches) -> String {
  format!("0.0.0.0:{}", cli.get_one::<String>("p2p-port").unwrap())
}

fn get_api_addr(cli: ArgMatches) -> String {
  format!("0.0.0.0:{}", cli.get_one::<String>("api-port").unwrap())
}

fn get_config() -> Result<Config, Box<dyn Error>> {
  match fs::read_to_string("config.toml") {
    Ok(content) => {
      let config: Config = toml::from_str(&content)?;
      Ok(config)
    },
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      Ok(Config {
        peers: vec![],
      })
    },
    Err(e) => Err(Box::new(e)),
  }
}

fn cli() -> Command {
  Command::new("Cryptogram")
    .version("1.0")
    .about("A decentralized microblogging platform on blockchain.")
    .args([
      Arg::new("p2p-port")
        .long("p2p-port")
        .help("The node port")
        .default_value("5000")
        .required(false),
      Arg::new("api-port")
        .long("api-port")
        .help("The API port")
        .default_value("3030")
        .required(false),
    ])
}
