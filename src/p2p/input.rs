use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::p2p::node::Node;
use crate::p2p::message::MessageData;

pub async fn handle_user_input(node: Arc<Node>) {
  let mut reader = BufReader::new(tokio::io::stdin());

  loop {
    let mut input = String::new();
    reader.read_line(&mut input).await.unwrap();
    let input = input.trim().to_string();

    match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
      ["/send", message @ ..] => {
        node.yell(&MessageData::Chat {
          message: message.join(" "),
        }).await;
      },
      ["/connect", peer] => {
        handle_peer_connect(node.clone(), peer).await;
      }
      ["/peers"] => {
        handle_peer_listing(node.clone()).await;
      }
      ["/sync"] => {
        handle_chain_syncing(node.clone()).await;
      },
      ["/chain"] => {
        handle_chain_listing(node.clone()).await;
      },
      ["/save"] => {
        node.chain
          .lock()
          .await
          .save_to_file("blockchain.json");

        println!("Saved blockchain to disk.");
      },
      _ => {
        println!("Commands:");
        println!("  /send <MESSAGE> - Broadcast a message to all peers");
        println!("  /connect <IP:PORT> - Manually connect to a peer");
        println!("  /peers - List connected peers");
        println!("  /sync - Sync the blockchain");
        println!("  /chain - List the blockchain contents");
      }
    }
  }
}

/**
 * Handle connecting to a peer.
 */
async fn handle_peer_connect(node: Arc<Node>, peer: &str) {
  let peer = peer.to_string();

  println!("Connected to {}", peer);

  node.add_peer(&peer).await;

  // Ask peer for its peers and blockchain.
  let chain_at = node.chain
    .lock()
    .await
    .len();

  node.send(&peer, &MessageData::PeerDiscovery {}).await;
  node.send(&peer, &MessageData::BlockRequest {index: chain_at}).await;
}

/**
 * Handle listing connected peers.
 */
async fn handle_peer_listing(node: Arc<Node>) {
  let peers_guard = node.peers.lock().await;

  if peers_guard.is_empty() {
    println!("No connected peers.");
  } else {
    println!("Connected peers:");
    for peer in peers_guard.iter() {
      println!("- {}", peer);
    }
  }
}

/**
 * Handle listing blockchain contents.
 */
async fn handle_chain_listing(node: Arc<Node>) {
  let chain = node.chain.lock()
    .await
    .to_json(true);

  println!("{}", chain);
}

/**
 * Handle syncing blockchain. This will pick a random peer and request
 * the entire blockchain from.
 */
async fn handle_chain_syncing(node: Arc<Node>) {
  let peer = node.get_random_peer().await.unwrap();

  node.send(&peer, &MessageData::BlockRequest {
    index: node.chain
      .lock()
      .await
      .len(),
  }).await;

  println!("requesting blockchain sync");
}
