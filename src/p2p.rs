use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    Chat,
    PeerDiscovery,
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

    println!("addr: {}", listener.local_addr().unwrap());

    // let (tx, mut _rx) = mpsc::unbounded_channel::<String>();

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

                    peers.lock()
                        .await
                        .insert(peer);

                    drop(stream);
                } else {
                    println!("Could not connect to peer: {}", peer);
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
        println!("[{}] {}", peer_addr, buffer.trim());
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
