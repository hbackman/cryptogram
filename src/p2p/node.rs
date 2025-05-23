use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;
use serde_json;
use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::sync::Arc;
use std::collections::HashMap;
use rand::seq::IteratorRandom;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use uuid::Uuid;
use crate::p2p::message::Message;
use crate::p2p::message::MessageData;
use crate::p2p::message::Handshake;
use crate::p2p::peer::Peer;
use crate::blockchain::chain::Blockchain;

// type Peer = UnboundedSender<Message>;

#[derive(Debug, Clone)]
pub struct Node {
  pub node_id:  String,
  pub peers:    Arc<Mutex<HashMap<String, Peer>>>,
  pub chain:    Arc<Mutex<Blockchain>>,
  pub listener: Arc<TcpListener>,
}

impl Node {
  pub async fn new(chain: Arc<Mutex<Blockchain>>, addr: String) -> Self {
    let node_id = Uuid::new_v4().to_string();

    println!("Running P2P on {}, Node ID: {}", addr, node_id);

    let listener = TcpListener::bind(&addr)
      .await
      .unwrap();

    Node {
      node_id,
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
   * Connect to a peer using their address.
   */
  pub async fn connect_to_peer(&self, peer_addr: String) -> Result<String, Box<dyn Error>> {
    let stream = TcpStream::connect(peer_addr).await?;
    let addr = stream.peer_addr()?.to_string();

    let (
      mut reader,
      mut writer,
    ) = stream.into_split();

    self.send_handshake(&mut writer).await?;

    let handshake = self.recv_handshake(&mut reader).await?;

    self.setup_peer(
      handshake.peer_id.clone(),
      addr,
      reader,
      writer,
    ).await;

    Ok(handshake.peer_id.clone())
  }

  /**
   * Handle an incoming peer connection.
   */
  pub async fn handle_incoming(&self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let (
      mut reader,
      mut writer,
    ) = stream.into_split();

    let handshake = self.recv_handshake(&mut reader).await?;

    self.send_handshake(&mut writer).await?;

    self.setup_peer(
      handshake.peer_id,
      handshake.addr,
      reader,
      writer,
    ).await;

    Ok(())
  }

  /**
   * Configure the communication channel for a peer.
   */
  async fn setup_peer(&self, peer_name: String, peer_addr: String, reader: OwnedReadHalf, mut writer: OwnedWriteHalf) {
    let (tx, mut rx): (
      UnboundedSender<Message>,
      UnboundedReceiver<Message>,
    ) = unbounded_channel();

    let peer = Peer::new(
      peer_name.clone(),
      peer_addr.clone(),
      tx.clone(),
    );

    self.peers
      .lock()
      .await
      .insert(peer.peer_name.clone(), peer);

    let peer_clone = peer_name.clone();
    let node_clone = self.clone();

    tokio::spawn(async move {
      while let Some(msg) = rx.recv().await {
        if let Ok(data) = serde_json::to_string(&msg) {
          writer.write_all(data.as_bytes()).await.unwrap();
          writer.write_all(b"\n").await.unwrap();

          if writer.flush().await.is_err() {
            println!("Disconnected from peer");

            node_clone.rem_peer(&peer_clone).await;

            break;
          }
        }
      }
    });

    let self_clone = self.clone();

    tokio::spawn(async move {
      let mut reader = BufReader::new(reader);
      let mut buffer = String::new();

      while reader.read_line(&mut buffer).await.unwrap() > 0 {
        if let Ok(message) = serde_json::from_str::<Message>(&buffer.trim()) {
          self_clone.handle_message(message).await;
        }
        buffer.clear();
      }
    });
  }

  async fn handle_message(&self, message: Message) {
    match message.payload {
      MessageData::Chat { message: msg } => {
        println!("[{}] {}", message.sender.unwrap(), msg);
      },
      MessageData::PeerDiscovery {} => {
        // self.send(&message.sender, &MessageData::PeerGossip {
        //   peers: self.get_peers().await,
        // }).await;
      },
      MessageData::PeerGossip { peers } => {
        println!("{:?}", peers);

//        for peer in peers {
//          self.connect_to_peer(
//            peer.addr,
//            Some(peer.name),
//          ).await;
//
//          // if ! self.has_peer(&peer.name).await {
//          //   let _ = self.connect_to_peer(&peer.addr).await;
//          // }
//        }
      },
      MessageData::BlockchainTx { block } => {
        println!("BlockchainTx: {:?}", block);

        self.chain
          .lock()
          .await
          .add_block(block)
          .unwrap_or_else(|e| println!("{}", e));
      },
      // When another node asks for a block, reply with the block at the index
      // which the node asked for.
      MessageData::BlockRequest { index } => {
        println!("BlockRequest: {:?}", index);

        let block = self.chain
          .lock()
          .await
          .at(index);

        if let Some(block) = block {
          self.send(&message.sender.unwrap(), &MessageData::BlockResponse { block }).await;
        }
      },
      // When receiving a block, add it to the chain and ask a random peer for
      // the next block. This will loop back until the chain is synced.
      MessageData::BlockResponse { block } => {
        println!("BlockRequest: {:?}", block);

        self.chain
          .lock()
          .await
          .add_block(block.clone())
          .unwrap_or_else(|e| println!("{}", e));

        let peer = self.get_random_peer()
          .await
          .unwrap();

        self.send(&peer, &MessageData::BlockRequest {
          index: (block.index as usize) + 1,
        }).await;
      },
      _ => {
        eprintln!("Unknown message.");
      },
    }
  }

  /**
   * Check if a peer exists.
   */
  pub async fn has_peer(&self, peer: &str) -> bool {
    if peer == self.node_id {
      return true;
    }

    self.peers.lock().await.contains_key(peer)
  }

  /**
   * Remove a node peer.
   */
  pub async fn rem_peer(&self, peer: &str) {
    self.peers.lock().await.remove(peer);
  }

  /**
   * Retrieve the node peers.
   */
  pub async fn get_peers(&self) -> Vec<Peer> {
    self.peers.lock().await
      .values()
      .cloned()
      .collect()
  }

  /**
   * Retrive a random peer.
   */
  pub async fn get_random_peer(&self) -> Option<String> {
    let peers = self.peers
      .lock()
      .await;

    // Pick a random peer from the HashSet
    peers.keys().choose(&mut rand::thread_rng()).cloned()
  }

  /**
   * Send message to a peer.
   */
  pub async fn send(&self, peer: &str, payload: &MessageData) {
    let message = Message {
      payload: payload.to_owned(),
      sender: Some(self.node_id.clone()),
      receiver: Some(peer.to_string()),
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
      self.send(&peer.peer_name, payload).await;
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

  async fn send_handshake(&self, writer: &mut OwnedWriteHalf) -> Result<(), Box<dyn Error>> {
    let sending = Handshake {
      version: "1".to_string(),
      peer_id: self.node_id.clone(),
      addr:    self.get_local_addr(),
    };

    // Send handshake
    writer.write_all(serde_json::to_string(&sending)?.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    Ok(())
  }

  async fn recv_handshake(&self, reader: &mut OwnedReadHalf) -> Result<Handshake, Box<dyn Error>> {
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();

    reader.read_line(&mut buffer).await.unwrap();
    let handshake = serde_json::from_str::<Handshake>(&buffer.trim()).unwrap();
    buffer.clear();

    // Validate handshake.
    if handshake.version != "1" {
      return Err("Invalid handshake version".into());
    }

    if handshake.peer_id == self.node_id {
      return Err("Cannot connect to self".into());
    }

    Ok(handshake)
  }

}
