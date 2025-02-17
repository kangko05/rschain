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
use tokio::sync::{mpsc, RwLock, RwLockWriteGuard};
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::NetMessage;
use super::{errors::NetResult, NetOps};
use crate::blockchain::{Block, Chain, Transaction};

pub type PeersMapType = HashMap<String, TcpStream>; // ipaddr, stream

pub struct BootstrapNode {
    uuid: String,
    socket_addr: String,
    chain: Chain,
    peers: Arc<RwLock<PeersMapType>>,
}

const BOOTSTRAP_PORT: u16 = 8000;

impl BootstrapNode {
    pub fn new() -> Self {
        let socket_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), BOOTSTRAP_PORT);

        Self {
            uuid: Uuid::new_v4().to_string(),
            chain: Self::init_chain(),
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
impl BootstrapNode {
    pub async fn run(&self) -> NetResult<()> {
        println!("running bootstrap node at {}", self.socket_addr);

        let (tx, rx) = mpsc::channel::<(NetMessage, TcpStream)>(32);

        let listen_handle = NetOps::listen(&self.socket_addr, tx);
        let msg_handle = self.handle_msg(rx);

        let _ = tokio::join!(listen_handle, msg_handle);

        Ok(())
    }

    fn handle_msg(&self, mut rx: mpsc::Receiver<(NetMessage, TcpStream)>) -> JoinHandle<()> {
        let peers = Arc::clone(&self.peers);
        let blocks = self.chain.get_blocks().clone();
        tokio::spawn(async move {
            while let Some((msg, stream)) = rx.recv().await {
                match msg {
                    NetMessage::GetChain => {
                        let mut stream = stream;
                        if let Err(err) = NetOps::write_message(&mut stream, &blocks).await {
                            eprintln!("failed to write response: {err}");
                        }
                    }

                    NetMessage::GetPeers => {
                        if let Ok(addr) = stream.peer_addr() {
                            // broad cast newnode
                            let w_peers = peers.write().await;
                            let mut w_peers = Self::broadcast_to_peers(w_peers, &msg).await;

                            // insert new node
                            w_peers.insert(addr.to_string(), stream);
                        } else {
                            eprintln!("faild to get address from the stream");
                        };
                    }

                    _ => eprintln!("msg ignored"), // TODO: implement display for msg to specify
                                                   // the ignored msg type
                }
            }
        })
    }

    async fn broadcast_to_peers<'a>(
        mut w_peers: RwLockWriteGuard<'a, PeersMapType>,
        msg: &NetMessage,
    ) -> RwLockWriteGuard<'a, PeersMapType> {
        for peer_stream in w_peers.values_mut() {
            if let Err(err) = NetOps::write_message(peer_stream, msg).await {
                eprintln!("failed to write new node msg: {err}")
            };
        }

        w_peers
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
