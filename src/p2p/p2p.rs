use tokio::sync::Mutex;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::{io, io::AsyncBufReadExt, select};
use crate::blockchain::chain::Blockchain;
use crate::blockchain::block::{Block, PendingBlock};
use crate::p2p::message::Message;
use crate::p2p::message::MessageData;
use crate::p2p::service::{P2PService, P2PEvent};
use super::service::P2PCommand;

/**
 * Start the p2p node.
 */
pub async fn start_p2p(chain: Arc<Mutex<Blockchain>>, port: u16) {
  let mut p2p = P2PService::new("test-new", port)
    .await
    .unwrap();

  let mut stdin = io::BufReader::new(io::stdin()).lines();

  loop {
    select! {
      Some(block) = next_mpool_block(chain.clone()) => {
        handle_block(chain.clone(), &p2p, block).await;
      },
      Some(event) = p2p.next_event() => {
        handle_event(chain.clone(), &p2p, event).await;
      },
      Ok(Some(line)) = stdin.next_line() => {
        handle_input(chain.clone(), &p2p, line).await;
      },
    }
  }
}

async fn next_mpool_block(chain: Arc<Mutex<Blockchain>>) -> Option<PendingBlock> {
  let mut chain = chain
    .lock()
    .await;
  chain.mpool.pop()
}

async fn handle_block(chain: Arc<Mutex<Blockchain>>, p2p: &P2PService, pending_block: PendingBlock) {
  println!("Processing block");

  let mut chain = chain.lock().await;
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

  let _ = p2p.yell(MessageData::BlockchainTx {
    block,
  }).await;
}

async fn handle_input(chain: Arc<Mutex<Blockchain>>, p2p: &P2PService, input: String) {
  match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
    ["/s", message @ ..] => {
      let _ = p2p.yell(MessageData::Chat{
        message: message.join(" "),
      }).await;
    },
    ["/w", peer, msg @ ..] => {
      let _ = p2p.send(peer, MessageData::Chat{
        message: msg.join(" "),
      }).await;
    },
    ["/connect", addr] => {
      let _ = p2p.cmd(P2PCommand::Connect(addr.to_string())).await;
    },
    ["/chain"] => {
      handle_chain_listing(chain).await;
    },
    ["/peers"] => {
      p2p.cmd(P2PCommand::ListPeers).await;
    },
    _ => {
      println!("Commands:");
      println!("  /s <MESSAGE> - Broadcast a message to all peers");
      println!("  /w <PEER> <MESSAGE> - Send a message to a peer");
      println!("  /connect <IP:PORT> - Manually connect to a peer");
      println!("  /peers - List connected peers");
      println!("  /sync - Sync the blockchain");
      println!("  /chain - List the blockchain contents");
    }
  }
}

async fn handle_event(chain: Arc<Mutex<Blockchain>>, p2p: &P2PService, event: P2PEvent) {
  match event {
    P2PEvent::Message(_peer, msg) => {
      handle_message(chain, p2p, msg).await;
    }
    P2PEvent::Discovered(peer) => {
      eprintln!("Found peer: {}", peer);

      // I don't know why it's not connected yet.
      sleep(Duration::from_millis(100));

      let chain_at = chain
        .lock()
        .await
        .len();

      let _ = p2p.send(&peer.to_string(), MessageData::BlockRequest { index: chain_at + 1 }).await;
    }
    P2PEvent::ListenAddr(addr) => {
      println!("Listening on {}", addr);
    }
    _ => {}
  }
}

async fn handle_message(chain: Arc<Mutex<Blockchain>>, p2p: &P2PService, message: Message) {
  match message.payload {
    MessageData::Chat {message} => {
      println!("message: {}", message);
    },
    MessageData::BlockchainTx { block } => {
      println!("BlockchainTx: {:?}", block);

      chain
        .lock()
        .await
        .add_block(block)
        .unwrap_or_else(|e| println!("{}", e));
    },
    MessageData::BlockRequest { index } => {
      println!("BlockRequest: {:?}", index);

      let block = chain
        .lock()
        .await
        .at(index);

      if let Some(block) = block {
        let _ = p2p.send(&message.sender.unwrap(), MessageData::BlockResponse { block }).await;
      }
    },
    MessageData::BlockResponse { block } => {
      println!("BlockResponse: {:?}", block);

      chain
        .lock()
        .await
        .add_block(block.clone())
        .unwrap_or_else(|e| println!("{}", e));
    },
    _ => {}
  }
}

/**
 * Handle listing blockchain contents.
 */
async fn handle_chain_listing(chain: Arc<Mutex<Blockchain>>) {
  chain
    .lock()
    .await
    .print_chain();
}

// /**
//  * Handle pending blocks in the mempool.
//  */
// pub async fn handle_mempool_blocks(chain: Arc<Mutex<Blockchain>>, p2p: &P2PService) {
//   loop {
//     let block = {
//       let mut chain = chain
//         .lock()
//         .await;
//       chain.mpool.pop()
//     };
//
//     // todo: cross-node race conditions (retry)
//
//     if let Some(pending_block) = block {
//       println!("Processing block");
//
//       let mut chain = chain.lock().await;
//       let mut block = Block::next(
//         &chain.top_block(),
//         pending_block.data
//       );
//
//       block.timestamp  = pending_block.timestamp;
//       block.signature  = pending_block.signature;
//       block.public_key = pending_block.public_key;
//
//       block.mine_block();
//       chain.add_block(block.clone())
//         .unwrap_or_else(|e| println!("{}", e));
//
//       println!("Processed block: {:?}", block);
//
//       p2p.yell(MessageData::BlockchainTx {
//         block,
//       }).await;
//     }
//
//     sleep(Duration::from_secs(1)).await;
//   }
// }
