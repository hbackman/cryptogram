use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use serde::{Serialize, Deserialize};
use serde_json;
use std::sync::Arc;
use std::collections::HashSet;
use rand::seq::SliceRandom; // To pick random peers for gossip

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
  Chat,
  PeerDiscovery,
  PeerGossip,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
  pub msg_type: MessageType,
  pub sender: String,
  pub payload: String,
}

#[derive(Debug, Clone)]
struct Node {
  peers:    Arc<Mutex<HashSet<String>>>,
  listener: Arc<TcpListener>,
}

impl Node {
  pub async fn new(addr: String) -> Self {
    println!("Listening for messages on {}", addr);

    let listener = TcpListener::bind(&addr)
      .await
      .unwrap();

    Node {
      peers: Arc::new(Mutex::new(HashSet::new())),
      listener: Arc::new(listener),
    }
  }

  pub fn get_local_addr(&self) -> String {
    self.listener.local_addr().unwrap().to_string()
  }

  // Send message to a peer.
  pub async fn send(&self, message: &Message, peer: &str) {
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

  // Send message to all peers.
  pub async fn yell(&self, message: &Message) {
    let peers = self.peers.lock().await.clone();
    for peer in peers.iter() {
      self.send(message, peer).await;
    }
  }

  // Add peer.
  async fn add_peer(&self, peer: &str) {
    let mut peers_guard = self.peers.lock().await;

    // check that it isn't added and isn't itself.
    if !peers_guard.contains(peer) && peer != self.get_local_addr() {
      println!("Discovered new peer: {}", peer);
      peers_guard.insert(peer.to_string());
    }
  }
}

pub async fn start_p2p_server(addr: String) {
  let node = Arc::new(Node::new(addr).await);

  // Spawn a task to handle incoming messages
  tokio::spawn(handle_incoming_messages(node.clone()));

  // Spawn a task to start gossip.
  tokio::spawn(start_peer_gossip(node.clone()));

  // Spawn a task to handle interactive input.
  tokio::spawn(handle_interactive_input(node.clone())).await.unwrap();
}

async fn handle_interactive_input(node: Arc<Node>) {
  // Read user input
  let mut reader = BufReader::new(tokio::io::stdin());

  loop {
    let mut input = String::new();
    reader.read_line(&mut input).await.unwrap();
    let input = input.trim().to_string();

    match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
      ["/connect", peer] => {
        let peer = peer.to_string();

        node.add_peer(&peer).await;

        println!("Connected to {}", peer);

        let discovery_request = Message {
          msg_type: MessageType::PeerDiscovery,
          sender: node.listener.local_addr().unwrap().to_string(),
          payload: "".to_string(),
        };

        node.send(&discovery_request, &peer).await;
      }
      ["/peers"] => {
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
      ["/send", message @ ..] => {
        node.yell(&Message{
          msg_type: MessageType::Chat,
          sender: node.listener.local_addr().unwrap().to_string(),
          payload: message.join(" "),
        }).await;
      }
      _ => {
        println!("Commands:");
        println!("  /connect <IP:PORT> - Manually connect to a peer");
        println!("  /send <MESSAGE> - Broadcast a message to all peers");
        println!("  /peers - List connected peers");
      }
    }
  }
}

// Handle incoming messages and track peers
async fn handle_incoming_messages(node: Arc<Node>) {
  loop {
    let (socket, addr) = node.listener.accept().await.unwrap();
    let peer_addr = addr.to_string();
    tokio::spawn(handle_client(node.clone(), socket, peer_addr));
  }
}

// Read messages from a connected peer
async fn handle_client(node: Arc<Node>, socket: TcpStream, peer_addr: String) {
  let mut reader = BufReader::new(socket);
  let mut buffer = String::new();

  println!("peer connected: {}", peer_addr);

  while reader.read_line(&mut buffer).await.unwrap() > 0 {
    if let Ok(message) = serde_json::from_str::<Message>(&buffer.trim()) {
      let sender = message.sender.clone();

      node.clone().add_peer(&sender).await;

      handle_message(node.clone(), message.clone()).await;
    }
    buffer.clear();
  }
}

async fn handle_message(node: Arc<Node>, message: Message) {
  match message.msg_type {
    MessageType::Chat => {
      println!("[{}] {}", message.sender, message.payload);
    }
    MessageType::PeerDiscovery => {
      let gossip = &Message{
        msg_type: MessageType::PeerGossip,
        sender: node.get_local_addr(),
        payload: get_peers_json(node.peers.clone()).await.to_string(),
      };

      node.send(gossip, &message.sender).await;
    }
    MessageType::PeerGossip => {
      println!("peer gossip: {}", message.payload);

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
    }
  }
}

async fn start_peer_gossip(node: Arc<Node>) {
  loop {
    sleep(Duration::from_secs(10)).await; // Gossip every 10 seconds

    let peers_guard = node.peers.lock().await;
    let known_peers: Vec<String> = peers_guard.iter().cloned().collect();

    if known_peers.is_empty() {
      continue;
    }

    // Pick a random subset of peers (up to 3 peers)
    let gossip_targets: Vec<String> = known_peers
      .choose_multiple(&mut rand::thread_rng(), 3)
      .cloned()
      .collect();

    // Create a gossip message
    let gossip_message = Message {
        msg_type: MessageType::PeerGossip,
        sender: node.get_local_addr(), // Replace with actual address
        payload: serde_json::to_string(&known_peers).unwrap(),
    };

    drop(peers_guard); // Unlock before sending messages

    // Send gossip to selected peers
    for peer in gossip_targets {
      node.send(&gossip_message, &peer).await;
    }
  }
}

async fn get_peers_json(peers: Arc<Mutex<HashSet<String>>>) -> String {
  let peers_guard = peers.lock().await;

  // Convert to JSON
  serde_json::to_string(&peers_guard
    .iter()
    .cloned()
    .collect::<Vec<String>>()
  ).unwrap()
}
