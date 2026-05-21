use crate::gossip_protocol::GossipProtocol;
use crate::message_protocol::P2PMessage;
use crate::peer_manager::PeerManager;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use crate::{Block, Transaction, BlockchainState, BLEEPError}; // Adjust imports based on your structure

/// Represents a message sent between nodes in the P2P network
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum P2PMessage {
    NewBlock(Block),                // New block to add to the blockchain
    NewTransaction(Transaction),   // New transaction to add to the mempool
    BlockchainRequest,             // Request for the current blockchain state
    BlockchainResponse(Vec<Block>),// Response containing the current blockchain state
}

/// Represents a P2P Node
pub struct P2PNode {
    id: String,
    addr: SocketAddr,
    peers: Arc<Mutex<HashSet<SocketAddr>>>, // List of connected peers
    blockchain: Arc<Mutex<BlockchainState>>, // Shared blockchain state
}

impl P2PNode {
    /// Creates a new P2P node
    pub fn new(id: String, addr: SocketAddr, blockchain: Arc<Mutex<BlockchainState>>) -> Self {
        P2PNode {
            id,
            addr,
            peers: Arc::new(Mutex::new(HashSet::new())),
            blockchain,
        }
    }

    /// Starts the P2P server
    pub fn start(&self) {
        let listener = TcpListener::bind(self.addr).expect("Failed to bind to address");
        println!("Node {} listening on {}", self.id, self.addr);

        let peers = self.peers.clone();
        let blockchain = self.blockchain.clone();

        // Handle incoming connections
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let peers = peers.clone();
                        let blockchain = blockchain.clone();
                        thread::spawn(move || {
                            P2PNode::handle_connection(stream, peers, blockchain);
                        });
                    }
                    Err(e) => {
                        eprintln!("Connection failed: {}", e);
                    }
                }
            }
        });
    }

    /// Handles an incoming connection
    fn handle_connection(
        mut stream: TcpStream,
        peers: Arc<Mutex<HashSet<SocketAddr>>>,
        blockchain: Arc<Mutex<BlockchainState>>,
    ) {
        let peer_addr = stream.peer_addr().expect("Failed to get peer address");

        // Add peer to the peer list
        peers.lock().unwrap().insert(peer_addr);

        // Handle incoming messages
        let mut buffer = Vec::new();
        if stream.read_to_end(&mut buffer).is_ok() {
            if let Ok(message) = bincode::serde::decode_from_slice::<P2PMessage>(&buffer, bincode::config::standard()).map(|(v, _)| v) {
                match message {
                    P2PMessage::NewBlock(block) => {
                        println!("Received new block from {}: {:?}", peer_addr, block);
                        let mut blockchain = blockchain.lock().unwrap();
                        if let Err(e) = blockchain.add_block(block) {
                            eprintln!("Failed to add block: {}", e);
                        }
                    }
                    P2PMessage::NewTransaction(transaction) => {
                        println!("Received new transaction from {}: {:?}", peer_addr, transaction);
                        blockchain.lock().unwrap().add_transaction(transaction);
                    }
                    P2PMessage::BlockchainRequest => {
                        println!("Blockchain state requested by {}", peer_addr);
                        let blockchain = blockchain.lock().unwrap();
                        let response = P2PMessage::BlockchainResponse(blockchain.get_state());
                        let serialized = bincode::serde::encode_to_vec(&response, bincode::config::standard()).unwrap();
                        stream.write_all(&serialized).unwrap();
                    }
                    P2PMessage::BlockchainResponse(blocks) => {
                        println!("Received blockchain state from {}: {:?}", peer_addr, blocks);
                        let mut blockchain = blockchain.lock().unwrap();
                        for block in blocks {
                            if let Err(e) = blockchain.add_block(block) {
                                eprintln!("Failed to add block: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Sends a message to a peer
    pub fn send_message(&self, peer_addr: SocketAddr, message: P2PMessage) {
        if let Ok(mut stream) = TcpStream::connect(peer_addr) {
            let serialized = bincode::serde::encode_to_vec(&message, bincode::config::standard()).expect("Failed to serialize message");
            stream.write_all(&serialized).expect("Failed to send message");
        } else {
            eprintln!("Failed to connect to peer {}", peer_addr);
        }
    }

    /// Broadcasts a message to all connected peers
    pub fn broadcast_message(&self, message: P2PMessage) {
        let peers = self.peers.lock().unwrap();
        for peer in peers.iter() {
            self.send_message(*peer, message.clone());
        }
    }
}

pub fn broadcast_load_balance_signal(&self, energy_usage: u64) {
    // Implementation that broadcasts a load balancing command or signal across the network.
    log::info!("P2PNetwork: Broadcasting load balance signal with energy usage: {}", energy_usage);
    // Add actual broadcasting logic here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_p2p_network() {
        let blockchain1 = Arc::new(Mutex::new(BlockchainState::new()));
        let blockchain2 = Arc::new(Mutex::new(BlockchainState::new()));

        let node1 = P2PNode::new("Node1".to_string(), "127.0.0.1:8081".parse().unwrap(), blockchain1.clone());
        let node2 = P2PNode::new("Node2".to_string(), "127.0.0.1:8082".parse().unwrap(), blockchain2.clone());

        node1.start();
        node2.start();

        thread::sleep(Duration::from_secs(1)); // Give the nodes time to start

        // Node1 sends a block to Node2
        let block = Block::new(1, "0".to_string(), vec![]).unwrap();
        node1.send_message("127.0.0.1:8082".parse().unwrap(), P2PMessage::NewBlock(block));

        thread::sleep(Duration::from_secs(1)); // Give the message time to propagate

        assert_eq!(blockchain2.lock().unwrap().chain.len(), 2); // The genesis block + the new block
    }
}