use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use serde_json;
use std::sync::Arc;
use std::collections::HashSet;
use rand::seq::IteratorRandom;
use crate::p2p::message::{Message, MessageType};
use crate::p2p::gossip;
use crate::p2p::input;
use crate::block::{Block, Blockchain};

#[derive(Debug, Clone)]
pub struct Node {
  pub peers:    Arc<Mutex<HashSet<String>>>,
  pub chain:    Arc<Mutex<Blockchain>>,
  pub listener: Arc<TcpListener>,
}

impl Node {
  pub async fn new(addr: String) -> Self {
    println!("Listening for messages on {}", addr);

    let listener = TcpListener::bind(&addr)
      .await
      .unwrap();

    Node {
      peers: Arc::new(Mutex::new(HashSet::new())),
      chain: Arc::new(Mutex::new(Blockchain::new())),
      listener: Arc::new(listener),
    }
  }

  pub fn get_local_addr(&self) -> String {
    self.listener
      .local_addr()
      .unwrap()
      .to_string()
  }

  /**
   * Register a node peer.
   */
  pub async fn add_peer(&self, peer: &str) {
    let mut peers_guard = self.peers.lock().await;

    if !peers_guard.contains(peer) && peer != self.get_local_addr() {
      println!("Discovered new peer: {}", peer);
      peers_guard.insert(peer.to_string());
    }
  }

  /**
   * Retrieve the node peers.
   */
  pub async fn get_peers(&self) -> Vec<String> {
    self.peers.lock().await
      .iter()
      .cloned()
      .collect()
  }

  /**
   * Retrive a random peer.
   */
  pub async fn get_random_peer(&self) -> Option<String> {
      let peers_guard = self.peers.lock().await;

      // Pick a random peer from the HashSet
      peers_guard.iter().choose(&mut rand::thread_rng()).cloned()
  }

  /**
   * Send message to a peer.
   */
  pub async fn send(&self, peer: &str, message: &Message) {
    if let Ok(mut stream) = TcpStream::connect(peer).await {
      let json_msg = serde_json::to_string(&message).unwrap();

      if let Err(e) = stream.write_all(json_msg.as_bytes()).await {
        println!("Failed to send message to {}: {}", peer, e);
      } else {
        println!("Sent: {:?} -> {}", message, peer);
      }
    } else {
      println!("Could not connect to peer: {}", peer);
    }
  }

  /**
   * Send message to all peers.
   */
  pub async fn yell(&self, message: &Message) {
    let peers = self.peers.lock().await.clone();
    for peer in peers.iter() {
      self.send(peer, message).await;
    }
  }
}

/**
 * Start the p2p node.
 */
pub async fn start_p2p_node(addr: String) {
  let node = Arc::new(Node::new(addr).await);

  tokio::spawn(handle_incoming_messages(node.clone()));
  tokio::spawn(gossip::handle_peer_gossip(node.clone()));
  tokio::spawn(input::handle_user_input(node.clone())).await.unwrap();
}

/**
 * Handle incoming messages and track peers.
 */
async fn handle_incoming_messages(node: Arc<Node>) {
  loop {
    let (socket, _) = node.listener.accept().await.unwrap();
    tokio::spawn(handle_client(node.clone(), socket));
  }
}

/**
 * Read messages from a connected peer.
 */
async fn handle_client(node: Arc<Node>, socket: TcpStream) {
  let mut reader = BufReader::new(socket);
  let mut buffer = String::new();

  while reader.read_line(&mut buffer).await.unwrap() > 0 {
    if let Ok(message) = serde_json::from_str::<Message>(&buffer.trim()) {
      let sender = message.sender.clone();

      node.clone().add_peer(&sender).await;

      handle_message(node.clone(), message.clone()).await;
    }
    buffer.clear();
  }
}

/**
 * Handle a peer message.
 */
async fn handle_message(node: Arc<Node>, message: Message) {
  match message.msg_type {
    MessageType::Chat => {
      println!("[{}] {}", message.sender, message.payload);
    }
    MessageType::PeerDiscovery => {
      let peers_json = serde_json::to_string(
        &node.get_peers().await
      ).unwrap();

      node.send(&message.sender, &Message{
        msg_type: MessageType::PeerGossip,
        sender: node.get_local_addr(),
        payload: peers_json.to_string(),
      }).await;
    }
    MessageType::PeerGossip => {
      match serde_json::from_str::<Vec<String>>(&message.payload) {
        Ok(new_peers) => {
          for peer in new_peers {
            node.add_peer(&peer).await;
          }
        }
        Err(e) => {
          println!("Failed to parse peer list: {}", e);
        }
      }
    },
    MessageType::BlockchainRequest => {
      println!("BlockchainRequest");

      let chain = node.chain.lock().await;

      node.send(&message.sender, &Message{
        msg_type: MessageType::BlockchainReply,
        sender: node.get_local_addr(),
        payload: chain.to_json(false),
      }).await;
    },
    MessageType::BlockchainReply => {
      println!("BlockchainReply");

      match serde_json::from_str::<Vec<Block>>(&message.payload) {
        Ok(new_chain) => {
          node.chain.lock()
            .await
            .update(new_chain);

          println!("Updated blockchain.");
        }
        Err(e) => {
          println!("Failed to parse blockchain: {}", e);
        }
      }
    },
    MessageType::BlockchainTx => {
      println!("BlockchainTx");

       match serde_json::from_str::<Block>(&message.payload) {
         Ok(block) => {
           node.chain
             .lock()
             .await
             .add_block(block);
         },
         Err(e) => {
           println!("Failed to parse block: {}", e);
         }
       }
    },
    _ => {}
  }
}
