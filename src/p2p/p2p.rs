use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::p2p::node::Node;
use crate::p2p::gossip;
use crate::p2p::input;
use crate::blockchain::block::Block;
use crate::blockchain::chain::Blockchain;
use crate::p2p::message::{Message, MessageType};

/**
 * Start the p2p node.
 */
pub async fn start_p2p(chain: Arc<Mutex<Blockchain>>, addr: String) {
  let node = Arc::new(Node::new(chain, addr).await);

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
      match serde_json::from_str::<Vec<Block>>(&message.payload) {
        Ok(new_chain) => {
          node.chain.lock()
            .await
            .update(new_chain);

          println!("BlockchainReply: Updated blockchain.");
        }
        Err(e) => {
          println!("BlockchainReply: Parsing Error: {}", e);
        }
      }
    },
    MessageType::BlockchainTx => {
       match serde_json::from_str::<Block>(&message.payload) {
         Ok(block) => {
           println!("BlockchainTx: {:?}", block);

           node.chain
             .lock()
             .await
             .add_block(block);
         },
         Err(e) => {
           println!("BlockchainTx: Parsing Error: {}", e);
         }
       }
    },
  }
}
