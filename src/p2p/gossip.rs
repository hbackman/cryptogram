use tokio::time::{sleep, Duration};
use std::sync::Arc;
use rand::seq::SliceRandom;
use crate::p2p::node::Node;
use crate::p2p::message::MessageData;

pub async fn handle_peer_gossip(node: Arc<Node>) {
  loop {
    sleep(Duration::from_secs(10)).await; // Gossip every 10 seconds

    let peers_guard = node.peers.lock().await;
    let known_peers: Vec<String> = peers_guard.iter().cloned().collect();

    if known_peers.is_empty() {
      continue;
    }

    // Pick a random subset of peers (up to 3 peers)
    let gossip_targets: Vec<String> = known_peers
      .choose_multiple(&mut rand::thread_rng(), 3)
      .cloned()
      .collect();

    // Create a gossip message
    let gossip_message = MessageData::PeerGossip {
      peers: known_peers,
    };

    drop(peers_guard); // Unlock before sending messages

    // Send gossip to selected peers
    for peer in gossip_targets {
      node.send(&peer, &gossip_message).await;
    }
  }
}
