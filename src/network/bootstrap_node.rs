#![allow(dead_code)]

/*
 * BootstrapNode
 * - responsible for initializing & starting the chain
 * - also, this node will be entry point of the network
 *
 * 1. init chain
 *    - create genesis block
 *    - setup initial chain state
 *
 * 2. requests to be handled - responses will be broadcasted to current peers
 *    - get peers -> respond with current peers list
 *    - get chain -> respond with current chain
 *    - new peer connection -> validate & add to peers list
 *    - new block notification -> validate & update chain
 *
 * 3. manage peers
 *    - implement ping/heartbeat -> at NetOps
 *    - maintain active peers list -> through ping
 *    - remove inactive peers -> through ping
 *    - broadcast important updates to all peers
 *    - handle peer discovery
 */

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use super::NetMessage;
use super::{errors::NetResult, NetOps};
use super::{NodeOps, NodeType, PeersMapType, RunNode};
use crate::blockchain::{Block, Chain, Transaction};

pub struct BootstrapNode {
    uuid: String,
    socket_addr: String,
    chain: Arc<RwLock<Chain>>,
    peers: Arc<RwLock<PeersMapType>>,
}

pub const BOOTSTRAP_PORT: u16 = 8000;

impl BootstrapNode {
    pub fn new() -> Self {
        let socket_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), BOOTSTRAP_PORT);

        Self {
            uuid: Uuid::new_v4().to_string(),
            chain: Arc::new(RwLock::new(Self::init_chain())),
            socket_addr,
            peers: Arc::new(RwLock::new(HashMap::<String, TcpStream>::new())),
        }
    }

    // failing to init chain -> panic
    fn init_chain() -> Chain {
        let mut chain = Chain::new();

        // first tx
        let tx = Transaction::coinbase("genesis tx", chain.get_block_height())
            .expect("failed to init tx");

        // genesis block
        let mut blk = Block::new("genesis block", &[tx]).expect("failed to init block");
        blk.mine().expect("failed to mine block");

        chain.genesis(&blk).expect("failed to add block to chain");

        chain
    }
}

// runner
impl RunNode for BootstrapNode {
    async fn run(&self) -> NetResult<()> {
        let (tx, rx) = mpsc::channel::<(NetMessage, TcpStream)>(32);

        let listen_handle = NetOps::listen(&self.socket_addr, tx);
        let msg_handle =
            NodeOps::handle_messages(NodeType::Bootstrap, &self.peers, &self.chain, rx, None);

        let _ = tokio::join!(listen_handle, msg_handle);

        Ok(())
    }
}
