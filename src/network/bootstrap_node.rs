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
 *    - implement ping/heartbeat
 *    - maintain active peers list
 *    - remove inactive peers
 *    - broadcast important updates to all peers
 *    - handle peer discovery
 */

use std::net::{IpAddr, Ipv4Addr};

use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::peer::PeersMap;
use super::NetMessage;
use super::{errors::NetResult, NetOps};
use crate::blockchain::{Block, Chain, Transaction};

pub struct BootstrapNode {
    uuid: String,
    socket_addr: String,
    chain: Chain,
    peers: PeersMap,
}

const BOOTSTRAP_PORT: u16 = 8000;

impl BootstrapNode {
    pub fn new() -> Self {
        let socket_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), BOOTSTRAP_PORT);

        Self {
            uuid: Uuid::new_v4().to_string(),
            chain: Self::init_chain(),
            socket_addr,
            peers: PeersMap::new(),
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
impl BootstrapNode {
    pub async fn run(&self) -> NetResult<()> {
        println!("running bootstrap node at {}", self.socket_addr);

        let (tx, rx) = mpsc::channel::<(NetMessage, TcpStream)>(32);

        let peers = self.peers.get_addr_vec();
        let listen_handle = NetOps::listen(&self.socket_addr, tx);
        let msg_handle = NetOps::abc(rx, move |msg, stream| {
            Self::handle_msg(msg, stream, peers.to_vec());
        });

        let _ = tokio::join!(listen_handle, msg_handle);

        Ok(())
    }

    async fn handle_msg(msg: NetMessage, mut stream: TcpStream, peers_addrs: Vec<String>) {
        match msg {
            NetMessage::GetPeers => {
                let payload = serde_json::to_vec(&json!(NetMessage::Peers(peers_addrs))).unwrap();
                NetOps::write_message(&mut stream, payload);
            }
            NetMessage::GetChain => {}
            _ => {}
        }
    }
}

#[cfg(test)]
mod bootstrap_test {
    use super::*;

    #[test]
    fn new() {
        let _ = BootstrapNode::new();
    }
}
