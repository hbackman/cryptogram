use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use serde_json;
use std::sync::Arc;
use std::collections::HashSet;
use rand::seq::IteratorRandom;
use crate::p2p::message::Message;
use crate::p2p::message::MessageData;
use crate::blockchain::chain::Blockchain;

#[derive(Debug, Clone)]
pub struct Node {
  pub peers:    Arc<Mutex<HashSet<String>>>,
  pub chain:    Arc<Mutex<Blockchain>>,
  pub listener: Arc<TcpListener>,
}

impl Node {
  pub async fn new(chain: Arc<Mutex<Blockchain>>, addr: String) -> Self {
    println!("Running p2p on {}", addr);

    let listener = TcpListener::bind(&addr)
      .await
      .unwrap();

    Node {
      peers:    Arc::new(Mutex::new(HashSet::new())),
      listener: Arc::new(listener),
      chain,
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
   * Remove a node peer.
   */
  pub async fn rem_peer(&self, peer: &str) {
    let mut peers_guard = self.peers.lock().await;

    peers_guard.remove(peer);
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
  pub async fn send(&self, peer: &str, payload: &MessageData) {
    let message = Message {
      payload: payload.to_owned(),
      sender: self.get_local_addr(),
    };

    if let Ok(mut stream) = TcpStream::connect(peer).await {
      let json_msg = serde_json::to_string(&message).unwrap();

      if let Err(e) = stream.write_all(json_msg.as_bytes()).await {
        println!("Failed to send message to {}: {}", peer, e);
      } else {
      }
    } else {
      self.rem_peer(peer).await;
      println!("Could not connect to peer: {}, removed from peer list.", peer);
    }
  }

  /**
   * Send message to all peers.
   */
  pub async fn yell(&self, payload: &MessageData) {
    let peers = self.peers.lock().await.clone();
    for peer in peers.iter() {
      self.send(peer, payload).await;
    }
  }
}
