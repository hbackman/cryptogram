use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::blockchain::block::Block;
use crate::p2p::node::Node;
use crate::p2p::message::{Message, MessageType};

pub async fn handle_user_input(node: Arc<Node>) {
  let mut reader = BufReader::new(tokio::io::stdin());

  loop {
    let mut input = String::new();
    reader.read_line(&mut input).await.unwrap();
    let input = input.trim().to_string();

    match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
      ["/send", message @ ..] => {
        node.yell(&Message{
          msg_type: MessageType::Chat,
          sender: node.get_local_addr(),
          payload: message.join(" "),
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
      ["/tx", message @ ..] => {
        handle_transaction(node.clone(), &message.join(" ")).await;
      },
      ["/chain"] => {
        handle_chain_listing(node.clone()).await;
      },
      _ => {
        println!("Commands:");
        println!("  /connect <IP:PORT> - Manually connect to a peer");
        println!("  /send <MESSAGE> - Broadcast a message to all peers");
        println!("  /peers - List connected peers");
        println!("  /sync - Sync the blockchain");
        println!("  /chain - List the blockchain contents");
        println!("  /tx <MESSAGE> - Add a blockchain transaction");
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

  // Ask peer for its peers.
  node.send(&peer, &Message{
    msg_type: MessageType::PeerDiscovery,
    sender: node.get_local_addr(),
    payload: "".to_string(),
  }).await;

  // Ask peer for its blockchain.
  node.send(&peer, &Message{
    msg_type: MessageType::BlockchainRequest,
    sender: node.get_local_addr(),
    payload: "".to_string(),
  }).await;
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

  node.send(&peer, &Message{
    msg_type: MessageType::BlockchainRequest,
    sender: node.get_local_addr(),
    payload: "".to_string(),
  }).await;

  println!("requesting blockchain sync");
}

/**
 * Handle new transaction.
 */
async fn handle_transaction(node: Arc<Node>, data: &str) {
  println!("mining new block");

  let mut chain = node.chain.lock().await;
  let mut block = Block::next(chain.latest_block(), data.to_string());

  block.mine_block();

  println!("mined new block");

  chain.add_block(block.clone());

  node.yell(&Message{
    msg_type: MessageType::BlockchainTx,
    sender: node.get_local_addr(),
    payload: block.to_json(),
  }).await;
}
