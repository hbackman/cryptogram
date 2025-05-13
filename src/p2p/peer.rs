use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::UnboundedSender;
use crate::p2p::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
  pub name: String,
  pub addr: String,
}

#[derive(Debug, Clone)]
pub struct Peer {
  pub peer_name: String,
  pub peer_addr: String,
  sender: UnboundedSender<Message>,
}

impl Peer {
  pub fn new(
    peer_name: String,
    peer_addr: String,
    sender: UnboundedSender<Message>
  ) -> Self {
    Self {
      peer_name,
      peer_addr,
      sender,
    }
  }

  pub fn send(&self, message: Message) {
    let _ = self.sender.send(message);
  }

  /**
   * Retrieve the peer information.
   */
  pub fn info(&self) -> PeerInfo {
    PeerInfo {
      name: self.peer_name.clone(),
      addr: self.peer_addr.clone(),
    }
  }
}
