use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::sync::Arc;
use std::time::Duration;
use crate::p2p::node::Node;
use crate::p2p::gossip;
use crate::p2p::input;
use crate::blockchain::block::Block;
use crate::blockchain::chain::Blockchain;
use crate::p2p::message::MessageData;

/**
 * Start the p2p node.
 */
pub async fn start_p2p(chain: Arc<Mutex<Blockchain>>, addr: String, peers: Vec<String>) {
  let node = Arc::new(Node::new(chain, addr).await);

  for peer in peers.clone() {
    let _ = node.connect_to_peer(&peer).await;
  }

  node.sync().await;

  let _ = tokio::join!(
    tokio::spawn(handle_mempool_blocks(node.clone())),
    tokio::spawn(handle_incoming_messages(node.clone())),
    tokio::spawn(gossip::handle_peer_gossip(node.clone())),
    tokio::spawn(input::handle_user_input(node.clone())),
  );
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
async fn handle_client(node: Arc<Node>, stream: TcpStream) {
  println!("incoming");

  let _ = node.handle_incoming(stream).await;
}

/**
 * Handle a peer message.
 */
// async fn handle_message(node: Arc<Node>, message: Message) {
//   match message.payload {
//     MessageData::Chat { message: msg } => {
//       println!("[{}] {}", message.sender, msg);
//     },
//     MessageData::PeerDiscovery {} => {
//       node.send(&message.sender, &MessageData::PeerGossip {
//         peers: node.get_peers().await,
//       }).await;
//     },
//     MessageData::PeerGossip { peers } => {
//       // for peer in peers {
//       //   node.add_peer(&peer).await;
//       // }
//     },
//     MessageData::BlockchainTx { block } => {
//       println!("BlockchainTx: {:?}", block);
//
//       node.chain
//         .lock()
//         .await
//         .add_block(block)
//         .unwrap_or_else(|e| println!("{}", e));
//     },
//
//     // When another node asks for a block, reply with the block at the index
//     // which the node asked for.
//     MessageData::BlockRequest { index } => {
//       println!("BlockRequest: {:?}", index);
//
//       let block = node.chain
//         .lock()
//         .await
//         .at(index);
//
//       if let Some(block) = block {
//         node.send(&message.sender, &MessageData::BlockResponse { block }).await;
//       }
//     },
//     // When receiving a block, add it to the chain and ask a random peer for
//     // the next block. This will loop back until the chain is synced.
//     MessageData::BlockResponse { block } => {
//       println!("BlockRequest: {:?}", block);
//
//       node.chain
//         .lock()
//         .await
//         .add_block(block.clone())
//         .unwrap_or_else(|e| println!("{}", e));
//
//       let peer = node.get_random_peer()
//         .await
//         .unwrap();
//
//       node.send(&peer, &MessageData::BlockRequest {
//         index: (block.index as usize) + 1,
//       }).await;
//     },
//     _ => {
//       eprintln!("Unknown message.");
//     },
//   }
// }

/**
 * Handle pending blocks in the mempool.
 */
pub async fn handle_mempool_blocks(node: Arc<Node>) {
  loop {
    let block = {
      let mut chain = node
        .chain
        .lock()
        .await;
      chain.mpool.pop()
    };

    // todo: cross-node race conditions (retry)

    if let Some(pending_block) = block {
      println!("Processing block");

      let mut chain = node.chain.lock().await;
      let mut block = Block::next(
        &chain.top_block(),
        pending_block.data
      );

      block.timestamp  = pending_block.timestamp;
      block.signature  = pending_block.signature;
      block.public_key = pending_block.public_key;

      block.mine_block();
      chain.add_block(block.clone())
        .unwrap_or_else(|e| println!("{}", e));

      println!("Processed block: {:?}", block);

      node.yell(&MessageData::BlockchainTx {
        block,
      }).await;
    }

    sleep(Duration::from_secs(1)).await;
  }
}
