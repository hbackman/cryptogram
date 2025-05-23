use tokio::sync::Mutex;
use std::sync::Arc;
use std::error::Error;
use tokio::{io, io::AsyncBufReadExt, select};
use crate::blockchain::chain::Blockchain;
use crate::p2p::message::MessageData;
use crate::p2p::service::{P2PService, P2PEvent};
use super::service::P2PCommand;

/**
 * Start the p2p node.
 */
pub async fn start_p2p(chain: Arc<Mutex<Blockchain>>, port: u16) -> Result<(), Box<dyn Error>> {
  let mut p2p = P2PService::new("test-new", port).await?;

  let mut stdin = io::BufReader::new(io::stdin()).lines();

  loop {
    select! {
      Ok(Some(line)) = stdin.next_line() => {
        handle_input(chain.clone(), &p2p, line).await;
      },
      Some(event) = p2p.next_event() => {
        handle_event(chain.clone(), &p2p, event).await;
      }
    }
  }
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
    }
    P2PEvent::ListenAddr(addr) => {
      println!("Listening on {}", addr);
    }
    _ => {}
  }
}

async fn handle_message(chain: Arc<Mutex<Blockchain>>, _p2p: &P2PService, message: MessageData) {
  match message {
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

      // let block = chain
      //   .lock()
      //   .await
      //   .at(index);

      // if let Some(block) = block {
      //   self.send(&message.sender, &MessageData::BlockResponse { block }).await;
      // }
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
// pub async fn handle_mempool_blocks(node: Arc<Node>) {
//   loop {
//     let block = {
//       let mut chain = node
//         .chain
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
//       let mut chain = node.chain.lock().await;
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
//       node.yell(&MessageData::BlockchainTx {
//         block,
//       }).await;
//     }
//
//     sleep(Duration::from_secs(1)).await;
//   }
// }
