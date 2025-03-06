use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use serde_json;

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

pub async fn start_p2p_server(addr: String) {
    println!("Listening for messages on {}", addr);

    let peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    let listener = Arc::new(
        TcpListener::bind(&addr)
            .await
            .unwrap()
    );

    // Spawn a task to handle incoming messages
    tokio::spawn(handle_incoming_messages(listener.clone(), peers.clone()));

    // Spawn a task to handle interactive input.
    tokio::spawn(handle_interactive_input(listener.clone(), peers.clone())).await.unwrap();
}

async fn handle_interactive_input(listener: Arc<TcpListener>, peers: Arc<Mutex<HashSet<String>>>) {
    // Read user input
    let mut reader = BufReader::new(tokio::io::stdin());

    loop {
        let mut input = String::new();
        reader.read_line(&mut input).await.unwrap();
        let input = input.trim().to_string();

        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["/connect", peer] => {
                let peer = peer.to_string();

                if let Ok(stream) = TcpStream::connect(&peer).await {
                    println!("Connected to {}", peer);

                    peers.lock().await.insert(peer.clone());

                    let discovery_request = Message {
                        msg_type: MessageType::PeerDiscovery,
                        sender: listener.local_addr().unwrap().to_string(),
                        payload: "".to_string(),
                    };

                    send_message(&peer, &discovery_request).await;

                    drop(stream);
                } else {
                    println!("Could not connect to peer: {}", peer);
                }
            }
            ["/peers"] => {
                let peers_guard = peers.lock().await;

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
                broadcast_message(peers.clone(), Message{
                    msg_type: MessageType::Chat,
                    sender: listener.local_addr().unwrap().to_string(),
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
async fn handle_incoming_messages(listener: Arc<TcpListener>, peers: Arc<Mutex<HashSet<String>>>) {
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let peer_addr = addr.to_string();
        tokio::spawn(handle_client(socket, peer_addr, peers.clone()));
    }
}

// Read messages from a connected peer
async fn handle_client(socket: TcpStream, peer_addr: String, peers: Arc<Mutex<HashSet<String>>>) {
    let mut reader = BufReader::new(socket);
    let mut buffer = String::new();

    println!("peer connected: {}", peer_addr);

    while reader.read_line(&mut buffer).await.unwrap() > 0 {
        if let Ok(message) = serde_json::from_str::<Message>(&buffer.trim()) {
            peers.lock().await.insert(message.sender.clone());

            match message.msg_type {
                MessageType::Chat => {
                    println!("[{}] {}", message.sender, message.payload);
                }
                MessageType::PeerDiscovery => {
                    send_message(&message.sender, &Message{
                        msg_type: MessageType::PeerGossip,
                        sender: "127.0.0.1:5002".to_string(),
                        payload: get_peers_json(peers.clone()).await.to_string(),
                    }).await;
                }
                MessageType::PeerGossip => {
                    println!("discovery reply: {}", message.payload);
                    merge_peers(peers.clone(), &message.payload).await;
                }
            }
        }
        buffer.clear();
    }
}

// Broadcast message to all known peers
async fn broadcast_message(peers: Arc<Mutex<HashSet<String>>>, message: Message) {
    let peers = peers.lock().await.clone();
    for peer in peers.iter() {
        send_message(peer, &message).await;
    }
}

// Send message to a peer.
async fn send_message(peer: &str, message: &Message) {
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

async fn get_peers_json(peers: Arc<Mutex<HashSet<String>>>) -> String {
    let peers_guard = peers.lock().await;

    // Convert to JSON
    serde_json::to_string(&peers_guard
        .iter()
        .cloned()
        .collect::<Vec<String>>()
    ).unwrap()
}

async fn merge_peers(peers: Arc<Mutex<HashSet<String>>>, json_peers: &str) {
    match serde_json::from_str::<Vec<String>>(json_peers) {
        Ok(new_peers) => {
            let mut peers_guard = peers.lock().await;
            for peer in new_peers {
                if !peers_guard.contains(&peer) {
                    println!("Discovered new peer: {}", peer);
                    peers_guard.insert(peer);
                }
            }
        }
        Err(e) => {
            println!("Failed to parse peer list: {}", e);
        }
    }
}
