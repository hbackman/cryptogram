use std::str::FromStr;
use std::time::Duration;
use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};
use libp2p::futures::stream::StreamExt;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::PeerId;
use libp2p::SwarmBuilder;
use libp2p::gossipsub::{self, Topic};
use libp2p::mdns;
use libp2p::noise;
use libp2p::tcp;
use libp2p::yamux;
use tokio::{io, select};
use tokio::{
  sync::mpsc::{self, Receiver, Sender},
};
use crate::p2p::message::Message;
use crate::p2p::message::MessageData;

#[derive(NetworkBehaviour)]
struct CryptogramBehaviour {
  gossipsub: gossipsub::Behaviour,
  mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum P2PEvent {
  Message(PeerId, MessageData),
  Discovered(PeerId),
  Expired(PeerId),
  ListenAddr(String),
}

pub enum P2PCommand {
  Yell(MessageData),
  Send(MessageData, PeerId),
  ListPeers,
  Connect(String),
}

pub struct P2PService {
  cmd_tx: Sender<P2PCommand>,
  evt_rx: Receiver<P2PEvent>,
}

impl P2PService {
  pub async fn new(topic: &str, port: u16) -> anyhow::Result<Self> {
    let swarm = SwarmBuilder::with_new_identity()
      .with_tokio()
      .with_tcp(
          tcp::Config::default(),
          noise::Config::new,
          yamux::Config::default,
      )?
      .with_behaviour(|key| {
        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &gossipsub::Message| {
          let mut s = DefaultHasher::new();
          message.data.hash(&mut s);
          gossipsub::MessageId::from(s.finish().to_string())
        };

        // Set a custom gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
          .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
          .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
          // signing)
          .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
          .build()
          .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

        // build a gossipsub network behaviour
        let gossipsub = gossipsub::Behaviour::new(
          gossipsub::MessageAuthenticity::Signed(key.clone()),
          gossipsub_config,
        )?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

        Ok(CryptogramBehaviour { gossipsub, mdns })
      })?
      .build();

    let (cmd_tx, cmd_rx) = mpsc::channel::<P2PCommand>(32);
    let (evt_tx, evt_rx) = mpsc::channel::<P2PEvent>(32);

    let topic = gossipsub::IdentTopic::new(topic);

    P2PService::run(
      swarm,
      topic,
      port,
      cmd_rx,
      evt_tx,
    ).await?;

    Ok(P2PService {
      cmd_tx,
      evt_rx,
    })
  }

  async fn run(
    mut swarm:  libp2p::Swarm<CryptogramBehaviour>,
    topic:      libp2p::gossipsub::IdentTopic,
    port:       u16,
    mut cmd_rx: Receiver<P2PCommand>,
    evt_tx:     Sender<P2PEvent>,
  ) -> anyhow::Result<()> {
    // subscribes to our topic
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    // Listen on all interfaces and specified port (0 = any available port)
    swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", port).parse()?)?;

    println!("Peer ID: {}", swarm.local_peer_id().to_string());

    tokio::spawn(async move {
      loop {
        select! {
          // Handle incoming
          Some(cmd) = cmd_rx.recv() => match cmd {
            P2PCommand::Yell(message) => {
              let message = Message {
                payload:  message,
                sender:   Some(swarm.local_peer_id().to_string()),
                receiver: None,
              };

              if let Ok(json) = serde_json::to_string(&message) {
                let _ = swarm
                  .behaviour_mut()
                  .gossipsub
                  .publish(topic.clone(), json);
              }
            },
            P2PCommand::Send(message, peer) => {
              let message = Message {
                payload:  message,
                sender:   Some(swarm.local_peer_id().to_string()),
                receiver: Some(peer.to_string()),
              };

              if let Ok(json) = serde_json::to_string(&message) {
                let _ = swarm
                  .behaviour_mut()
                  .gossipsub
                  .publish(topic.clone(), json);
              }
            },
            P2PCommand::ListPeers => {
              let peers: Vec<String> = swarm
                .behaviour_mut()
                .gossipsub
                .all_peers()
                .map(|(peer_id, _topics)| peer_id.to_string())
                .collect();

              for peer in peers {
                println!("- {}", peer);
              }
            },
            P2PCommand::Connect(addr) => {
              match addr.parse::<libp2p::Multiaddr>() {
                Ok(multiaddr) => {
                  match swarm.dial(multiaddr) {
                    Ok(_) => println!("Dialing {}", addr),
                    Err(e) => println!("Failed to dial {}: {}", addr, e),
                  }
                },
                Err(e) => println!("Invalid address {}: {}", addr, e),
              }
            },
          },
          // Handle swarm events
          event = swarm.select_next_some() => match event {
            // connected
            SwarmEvent::Behaviour(
              CryptogramBehaviourEvent::Mdns(mdns::Event::Discovered(list))
            ) => {
              for (peer, _multiaddr) in list {
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);

                let _ = evt_tx.send(P2PEvent::Discovered(peer)).await;
              }
            },
            SwarmEvent::Behaviour(
              CryptogramBehaviourEvent::Mdns(mdns::Event::Expired(list))
            ) => {
              for (peer, _multiaddr) in list {
                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);

                let _ = evt_tx.send(P2PEvent::Expired(peer)).await;
              }
            },
            SwarmEvent::Behaviour(
              CryptogramBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                  propagation_source: peer_id,
                  message_id: _id,
                  message,
                }
              )
            ) => {
              let msg = serde_json::from_str::<Message>(
                &String::from_utf8_lossy(&message.data)
              ).unwrap();

              // Message was sent to everyone.
              if msg.receiver == None {
                let _ = evt_tx.send(P2PEvent::Message(peer_id, msg.payload.clone())).await;
              }

              // Message was sent to specific peer.
              if msg.receiver == Some(swarm.local_peer_id().to_string()) {
                let _ = evt_tx.send(P2PEvent::Message(peer_id, msg.payload.clone())).await;
              }
            },
            SwarmEvent::NewListenAddr { address, .. } => {
              let _ = evt_tx.send(P2PEvent::ListenAddr(address.to_string())).await;
            },
            _ => {}
          }
        }
      }
    });
    Ok(())
  }

  pub async fn cmd(&self, cmd: P2PCommand) {
    self.cmd_tx.send(cmd)
      .await
      .unwrap();
  }

  // Pull the next event (incoming message, peer‐found, etc.)
  pub async fn next_event(&mut self) -> Option<P2PEvent> {
    self.evt_rx.recv().await
  }

  /// Send a typed message to the network
  pub async fn yell(&self, msg: MessageData) -> anyhow::Result<()> {
    self.cmd_tx.send(P2PCommand::Yell(msg)).await?;
    Ok(())
  }

  /// Send a typed message to a specific peer.
  pub async fn send(&self, peer: &str, msg: MessageData) -> anyhow::Result<()> {
    self.cmd_tx.send(P2PCommand::Send(msg, PeerId::from_str(peer)?)).await?;
    Ok(())
  }
}
