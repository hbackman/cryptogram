use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;

pub async fn start_p2p_server(addr: String) {

    println!("Listening for messages on {}", addr);

    let peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    let listener = TcpListener::bind(&addr)
        .await
        .unwrap();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Spawn a task to handle incoming messages
    tokio::spawn(handle_incoming_messages(listener, tx.clone(), peers.clone()));

    // Read user input and broadcast messages
    let peers_clone = peers.clone();

    tokio::spawn(async move {
        // Read user input and send messages
        let mut reader = BufReader::new(tokio::io::stdin());

        loop {
            let mut input = String::new();
            reader.read_line(&mut input).await.unwrap();
            let input = input.trim().to_string();

            if input.starts_with("/connect") {
                let parts: Vec<&str> = input.split_whitespace().collect();
                if parts.len() != 2 {
                    println!("Usage: /connect <IP:PORT>");
                    continue;
                }
                let peer = parts[1].to_string();
                peers_clone.lock().await.insert(peer.clone());
                println!("Connected to {}", peer);
            } else if input.starts_with("/send") {
                let parts: Vec<&str> = input.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    println!("Usage: /send <MESSAGE>");
                    continue;
                }
                let message = parts[1].to_string();
                broadcast_message(peers_clone.clone(), message).await;
            } else {
                println!("Commands:");
                println!("  /send <MESSAGE> - Broadcast a message to all peers");
            }

            if let Ok(msg) = rx.try_recv() {
                println!("\n[RECEIVED] {}", msg);
            }
        }
    }).await.unwrap();
}

// Handle incoming messages and track peers
async fn handle_incoming_messages(listener: TcpListener, tx: mpsc::UnboundedSender<String>, peers: Arc<Mutex<HashSet<String>>>) {
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let peer_addr = addr.to_string();
        peers.lock().await.insert(peer_addr.clone());
        println!("New peer connected: {}", peer_addr);
        tokio::spawn(handle_client(socket, peer_addr, tx.clone(), peers.clone()));
    }
}

// Read messages from a connected peer
async fn handle_client(mut socket: TcpStream, peer_addr: String, tx: mpsc::UnboundedSender<String>, peers: Arc<Mutex<HashSet<String>>>) {
    let mut reader = BufReader::new(socket);
    let mut buffer = String::new();

    while reader.read_line(&mut buffer).await.unwrap() > 0 {
        let msg = format!("[{}] {}", peer_addr, buffer.trim());
        tx.send(msg.clone()).unwrap();
        println!("{}", msg);

        // Re-broadcast received messages to other peers
        broadcast_message(peers.clone(), buffer.trim().to_string()).await;

        buffer.clear();
    }
}

// Broadcast message to all known peers
async fn broadcast_message(peers: Arc<Mutex<HashSet<String>>>, message: String) {
    let peers = peers.lock().await.clone();
    for peer in peers.iter() {
        send_message(peer, &message).await;
    }
}

// Send a message to a specific peer
async fn send_message(peer: &str, message: &str) {
    if let Ok(mut stream) = TcpStream::connect(peer).await {
        let full_msg = format!("{}\n", message);
        if let Err(e) = stream.write_all(full_msg.as_bytes()).await {
            println!("Failed to send message to {}: {}", peer, e);
        } else {
            println!("Sent: '{}' -> {}", message, peer);
        }
    } else {
        println!("Could not connect to peer: {}", peer);
    }
}
