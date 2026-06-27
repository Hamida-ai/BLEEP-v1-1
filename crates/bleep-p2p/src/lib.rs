//! # bleep-p2p
//!
//! Production-grade P2P networking for the BLEEP blockchain ecosystem.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                      P2PNode                            │
//! │  ┌──────────────┐  ┌─────────────┐  ┌───────────────┐  │
//! │  │ PeerManager  │  │  Gossip     │  │ OnionRouter   │  │
//! │  │  + AI Score  │  │  Protocol   │  │ (dark routing)│  │
//! │  │  + Sybil Det │  │  Plumtree   │  │  Kyber/AES    │  │
//! │  └──────────────┘  └─────────────┘  └───────────────┘  │
//! │  ┌──────────────────────────────────────────────────┐   │
//! │  │              MessageProtocol                     │   │
//! │  │  TCP framing  ·  AES-256-GCM  ·  Ed25519 sig   │   │
//! │  │  Kyber-768 KEM  ·  Anti-replay nonce cache      │   │
//! │  └──────────────────────────────────────────────────┘   │
//! │  ┌──────────────────────────────────────────────────┐   │
//! │  │              KademliaDHT                         │   │
//! │  │  256 K-buckets  ·  XOR metric  ·  k=20          │   │
//! │  └──────────────────────────────────────────────────┘   │
//! │  ┌──────────────────────────────────────────────────┐   │
//! │  │              QuantumCrypto                       │   │
//! │  │  Kyber-768  ·  SPHINCS+-SHA2-128s  ·  Ed25519   │   │
//! │  └──────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```no_run
//! use bleep_p2p::p2p_node::{P2PNode, P2PNodeConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = P2PNodeConfig::default();
//!     let (node, handle) = P2PNode::start(config).await.unwrap();
//!     println!("Node started: {}", node.node_id);
//!     handle.shutdown().await;
//! }
//! ```

pub mod ai_security;
pub mod error;
pub mod gossip_protocol;
pub mod kademlia_dht;
pub mod message_protocol;
pub mod onion_routing;
pub mod p2p_node;
pub mod peer_manager;
pub mod quantum_crypto;
pub mod sig_availability;
pub mod types;

// Re-export the most commonly used items at crate root
pub use error::{P2PError, P2PResult};
pub use p2p_node::{NodeHandle, P2PNode, P2PNodeConfig};
pub use peer_manager::{PeerEvent, PeerManager, PeerManagerConfig};
pub use sig_availability::SigAvailabilityBridge;
pub use types::{MessageType, NodeId, PeerInfo, PeerStatus, SecureMessage};
