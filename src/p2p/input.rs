use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::p2p::node::Node;
use crate::p2p::message::{Message, MessageType};

pub async fn handle_user_input(node: Arc<Node>) {
  let mut reader = BufReader::new(tokio::io::stdin());

  loop {
    let mut input = String::new();
    reader.read_line(&mut input).await.unwrap();
    let input = input.trim().to_string();

    match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
      ["/connect", peer] => {
        let peer = peer.to_string();

        println!("Connected to {}", peer);

        node.add_peer(&peer).await;

        node.send(&peer, &Message{
          msg_type: MessageType::PeerDiscovery,
          sender: node.listener.local_addr().unwrap().to_string(),
          payload: "".to_string(),
        }).await;
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
          sender: node.get_local_addr(),
          payload: message.join(" "),
        }).await;
      },
      ["/sync"] => {
        // let chain = node.chain.lock().await;
        // let block = chain.latest_block();

        // node.yell(&Message{
        //   msg_type: MessageType::BlockchainSync,
        //   sender: node.get_local_addr(),
        //   payload: serde_json::to_string(&block).unwrap(),
        // });

        node.yell(&Message{
          msg_type: MessageType::GetBlockchainHeaders,
          sender: node.get_local_addr(),
          payload: "".to_string(),
        }).await;

        println!("requesting blockchain sync");
      },
      _ => {
        println!("Commands:");
        println!("  /connect <IP:PORT> - Manually connect to a peer");
        println!("  /send <MESSAGE> - Broadcast a message to all peers");
        println!("  /peers - List connected peers");
        println!("  /sync - Sync the blockchain");
      }
    }
  }
}
