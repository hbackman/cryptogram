use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufWriter, BufReader};
use tokio::sync::Mutex;
use serde_json;
use std::sync::Arc;
use std::collections::HashMap;
use rand::seq::IteratorRandom;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use crate::p2p::message::Message;
use crate::p2p::message::MessageData;
use crate::blockchain::chain::Blockchain;

type Peer = UnboundedSender<Message>;

#[derive(Debug, Clone)]
pub struct Node {
  pub peers:    Arc<Mutex<HashMap<String, Peer>>>,
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
      peers:    Arc::new(Mutex::new(HashMap::new())),
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
    if peer == self.get_local_addr() {
      return;
    }

    if self.has_peer(peer).await {
      return;
    }

    match TcpStream::connect(peer).await {
      Ok(stream) => {
        self.setup_peer(stream).await;
      },
      Err(e) => {
        println!("Failed to connect to {}: {}", peer, e);
      }
    }
  }

  pub async fn setup_peer(&self, stream: TcpStream) {
    let peer_addr = stream
      .peer_addr()
      .unwrap()
      .to_string();

    let (reader, mut writer) = stream.into_split();

    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();

    let (tx, mut rx): (
      UnboundedSender<Message>,
      UnboundedReceiver<Message>,
    ) = unbounded_channel();

    self.peers
      .lock()
      .await
      .insert(peer_addr.clone(), tx.clone());

    let node_clone = self.clone();

    tokio::spawn(async move {
      while let Some(msg) = rx.recv().await {
        if let Ok(data) = serde_json::to_string(&msg) {

          println!("sending: {:?}", msg);

          writer.write_all(data.as_bytes()).await.unwrap();
          writer.write_all(b"\n").await.unwrap();

          if writer.flush().await.is_err() {
            println!("Disconnected from peer {}", peer_addr);

            node_clone.rem_peer(&peer_addr).await;

            break;
          }
        }
      }
    });

    tokio::spawn(async move {
      while reader.read_line(&mut buffer).await.unwrap() > 0 {
        if let Ok(message) = serde_json::from_str::<Message>(&buffer.trim()) {
          let sender = message.sender.clone();

          println!("received {:?}", message);

        }
        buffer.clear();
      }
    });
  }

  pub async fn has_peer(&self, peer: &str) -> bool {
    self.peers.lock().await.contains_key(peer)
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
      .keys()
      .cloned()
      .collect()
  }

  /**
   * Retrive a random peer.
   */
  pub async fn get_random_peer(&self) -> Option<String> {
    let peers = self.peers.lock().await;

    // Pick a random peer from the HashSet
    peers.keys().choose(&mut rand::thread_rng()).cloned()
  }

  /**
   * Send message to a peer.
   */
  pub async fn send(&self, peer: &str, payload: &MessageData) {
    let message = Message {
      payload: payload.to_owned(),
      sender: self.get_local_addr(),
    };

    let peers = self.peers.lock().await;

    if let Some(sender) = peers.get(peer) {
      let _ = sender.send(message);
    } else {
      println!("No such peer: {}", peer);
    }
  }

  /**
   * Send message to all peers.
   */
  pub async fn yell(&self, payload: &MessageData) {
    for peer in self.get_peers().await {
      self.send(&peer, payload).await;
    }
  }

  /**
   * Sync the node with a random peer.
   */
  pub async fn sync(&self) {
    match self.get_random_peer().await {
      Some(peer) => {
        self.send(&peer, &MessageData::BlockRequest {
          index: self.chain
            .lock()
            .await
            .len(),
        }).await;

        println!("Requesting blockchain sync");
      }
      None => {}
    }
  }
}
