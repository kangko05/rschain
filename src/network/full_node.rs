#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::{errors::NetResult, NetOps};
use super::{NetMessage, BOOTSTRAP_PORT};
use super::{NodeOps, NodeType, PeersMapType};
use crate::blockchain::{Chain, TxPool, TxResult};
use crate::wallet::Wallet;

#[derive(Debug)]
pub struct FullNode {
    uuid: String,
    socket_addr: String,
    chain: Arc<RwLock<Chain>>,
    peers: Arc<RwLock<PeersMapType>>,
    mempool: Arc<RwLock<TxPool>>,
}

// constructor
impl FullNode {
    /// initializing full node depends on initial data from bootstrap node -> init
    pub async fn init(port: u16) -> NetResult<Self> {
        let socket_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        let bootstrap_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), BOOTSTRAP_PORT);

        let blocks = NodeOps::get_blocks(&bootstrap_addr).await?;
        let peers_addrs = NodeOps::get_peers(&bootstrap_addr).await?;
        let peers = NodeOps::connect_to_peers(peers_addrs).await;
        NodeOps::add_me_to_peer(&bootstrap_addr, &socket_addr).await?;

        Ok(Self {
            uuid: Uuid::new_v4().to_string(),
            chain: Arc::new(RwLock::new(Chain::from(blocks)?)),
            socket_addr,
            peers: Arc::new(RwLock::new(peers)),
            mempool: Arc::new(RwLock::new(TxPool::new())),
        })
    }
}

// TODO: to trait
impl FullNode {
    pub fn run(&self) -> NetResult<Vec<JoinHandle<()>>> {
        let (tx, rx) = mpsc::channel::<(NetMessage, TcpStream)>(32);

        let listen_handle = NetOps::listen(&self.socket_addr, tx);
        let msg_handle = NodeOps::handle_messages(
            NodeType::Full,
            &self.peers,
            &self.chain,
            rx,
            &self.mempool,
            None,
        );

        Ok(vec![listen_handle, msg_handle])
    }
}

impl FullNode {
    pub async fn send_tx(&self, from: &Wallet, to_addr: &str, value: u64) -> TxResult<()> {
        let tx = NodeOps::create_tx(&self.chain, from, to_addr, value).await?;
        NodeOps::broadcast_tx(&tx, &self.peers).await;

        Ok(())
    }
}

// getter
impl FullNode {
    pub fn get_addr(&self) -> &String {
        &self.socket_addr
    }

    pub fn get_peers(&self) -> &Arc<RwLock<PeersMapType>> {
        &self.peers
    }

    pub fn get_chain(&self) -> &Arc<RwLock<Chain>> {
        &self.chain
    }

    pub fn get_uuid(&self) -> &String {
        &self.uuid
    }

    pub fn get_mempool(&self) -> &Arc<RwLock<TxPool>> {
        &self.mempool
    }
}
