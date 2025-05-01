use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};

pub struct Peer {
  pub peer_id:   String,
  pub peer_addr: String,
  sender:   UnboundedSender<String>,
  receiver: UnboundedReceiver<String>,
}

impl Peer {
  pub fn new(peer_id: String, peer_addr: String) -> Self {
    let (sender, receiver) = unbounded_channel();

    Self {
      peer_id,
      peer_addr,
      sender,
      receiver,
    }
  }
}
