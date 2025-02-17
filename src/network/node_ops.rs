#![allow(dead_code)]

use super::errors::{NetError, NetResult};
use super::{NetMessage, NetOps};
use crate::blockchain::{Block, Chain, Transaction, TxResult};
use crate::wallet::Wallet;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock, RwLockWriteGuard};
use tokio::task::JoinHandle;

pub type PeersMapType = HashMap<String, TcpStream>; // ipaddr, stream

pub trait RunNode {
    async fn run(&self) -> NetResult<()>;
}

pub trait HandleMessage {
    fn handle_msg(&self, rx: mpsc::Receiver<(NetMessage, TcpStream)>) -> JoinHandle<()>;

    async fn broadcast_to_peers<'a>(
        w_peers: RwLockWriteGuard<'a, PeersMapType>,
        msg: &NetMessage,
    ) -> RwLockWriteGuard<'a, PeersMapType>;
}

#[derive(PartialEq)]
pub enum NodeType {
    Bootstrap,
    Full,
    Mining,
}

/// contains common operations of nodes
pub struct NodeOps;

// public - basic methods
impl NodeOps {
    pub fn create_tx(
        chain: &Chain,
        from: &Wallet,
        to_addr: &str,
        value: u64,
    ) -> TxResult<Transaction> {
        let utxos = chain.get_utxos();
        let from_outputs = chain.get_address_txs(from.get_address());

        Transaction::new(utxos, from, from_outputs, to_addr, value)
    }

    pub async fn broadcast_tx(tx: &Transaction, peers: &Arc<RwLock<PeersMapType>>) {
        let w_peers = peers.write().await;
        let _ = Self::broadcast_to_peers(w_peers, &NetMessage::NewTx(tx.clone())).await;
    }

    pub async fn broadcast_block(blk: &Block, peers: &Arc<RwLock<PeersMapType>>) {
        let w_peers = peers.write().await;
        let _ = Self::broadcast_to_peers(w_peers, &NetMessage::NewBlock(blk.clone())).await;
    }
}

// public - requests
impl NodeOps {
    pub async fn broadcast_to_peers<'a>(
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

    pub async fn get_blocks(bootstrap_addr: &str) -> NetResult<Vec<Block>> {
        let mut stream = TcpStream::connect(bootstrap_addr).await?;

        NetOps::write_message(&mut stream, NetMessage::GetChain).await?;

        let resp = NetOps::read_stream(&mut stream).await?;
        let parsed = serde_json::from_slice::<NetMessage>(&resp)?;

        match parsed {
            NetMessage::Blocks(blocks) => Ok(blocks),
            _ => Err(NetError::str("not a chain! expected blocks as response")),
        }
    }

    // expecting Vec<peer socket addr as String>
    pub async fn get_peers(bootstrap_addr: &str) -> NetResult<Vec<String>> {
        let mut stream = TcpStream::connect(&bootstrap_addr).await?;
        NetOps::write_message(&mut stream, NetMessage::GetPeers).await?;

        let resp = NetOps::read_stream(&mut stream).await?;
        let parsed = serde_json::from_slice::<NetMessage>(&resp)?;

        match parsed {
            NetMessage::Peers(peers) => Ok(peers),
            _ => Err(NetError::str("not a chain! expected blocks as response")),
        }
    }

    pub async fn connect_to_peers(peers_addrs: Vec<String>) -> PeersMapType {
        let mut peers = HashMap::<String, TcpStream>::new();

        if peers_addrs.is_empty() {
            return peers;
        }

        for addr in peers_addrs {
            if let Ok(stream) = TcpStream::connect(&addr).await {
                peers.insert(addr, stream);
            };
        }

        peers
    }

    pub async fn add_me_to_peer(bootstrap_addr: &str, my_addr: &str) -> NetResult<()> {
        let mut stream = TcpStream::connect(&bootstrap_addr).await?;

        NetOps::write_message(&mut stream, NetMessage::AddToPeers(my_addr.to_string())).await?;
        let resp = NetOps::read_stream(&mut stream).await?;
        let parsed = serde_json::from_slice::<NetMessage>(&resp)?;

        match parsed {
            NetMessage::Ok => Ok(()),
            _ => Err(NetError::str("expecting 'Ok' as a response")),
        }
    }
}

impl NodeOps {
    /// msg handler
    /// pass in tx for mining node
    /// pass in None for other nodes
    pub fn handle_messages(
        node_type: NodeType,
        peers: &Arc<RwLock<PeersMapType>>,
        chain: &Arc<RwLock<Chain>>,
        mut rx: mpsc::Receiver<(NetMessage, TcpStream)>,
        tx: Option<mpsc::Sender<Transaction>>,
    ) -> JoinHandle<()> {
        let peers = Arc::clone(peers);
        let chain = Arc::clone(chain);
        tokio::spawn(async move {
            let blocks = chain.read().await.get_blocks().clone();

            while let Some((msg, mut stream)) = rx.recv().await {
                match msg {
                    NetMessage::GetChain => {
                        println!("got getchain msg");

                        let mut stream = stream;
                        if let Err(err) =
                            NetOps::write_message(&mut stream, NetMessage::Blocks(blocks.clone()))
                                .await
                        {
                            eprintln!("failed to write response: {err}");
                        }
                    }

                    NetMessage::GetPeers => {
                        println!("got getpeers msg");

                        let r_peers = peers.read().await;
                        let peers_vec = r_peers
                            .iter()
                            .map(|(peer_addr, _)| peer_addr.to_string())
                            .collect::<Vec<_>>();

                        if let Err(err) =
                            NetOps::write_message(&mut stream, NetMessage::Peers(peers_vec)).await
                        {
                            eprintln!("failed to write peers response: {err}");
                        };
                    }

                    NetMessage::AddToPeers(addr) => {
                        if peers.read().await.contains_key(&addr) {
                            eprintln!("peer {} already in the list", addr);
                            return;
                        }

                        // broad cast newnode
                        let w_peers = peers.write().await;
                        let mut w_peers = Self::broadcast_to_peers(
                            w_peers,
                            &NetMessage::NewNode(addr.to_string()),
                        )
                        .await;

                        println!("adding {} to peers", addr);

                        if let Err(err) = NetOps::write_message(&mut stream, NetMessage::Ok).await {
                            eprintln!("boot: failed to write ok back: {err}");
                        };

                        // insert new node
                        w_peers.insert(addr, stream);
                    }

                    NetMessage::NewNode(addr) => {
                        if peers.read().await.contains_key(&addr) {
                            eprintln!("peer {} already in the list", addr);
                            return;
                        }

                        let mut w_peers = peers.write().await;

                        println!("adding {} to peers", addr);

                        if let Err(err) = NetOps::write_message(&mut stream, NetMessage::Ok).await {
                            eprintln!("boot: failed to write ok back: {err}");
                        };

                        w_peers.insert(addr, stream);
                    }

                    // spread to peers
                    // TODO: need to validate tx
                    //  - check utxo to prevent double spending
                    //  - check chain if its already in the chain
                    //  - check signature again
                    NetMessage::NewTx(transaction) => {
                        let w_peers = peers.write().await;
                        let _ = Self::broadcast_to_peers(
                            w_peers,
                            &NetMessage::NewTx(transaction.clone()),
                        )
                        .await;

                        if node_type.eq(&NodeType::Mining) {
                            match &tx {
                                Some(sender) => {
                                    if let Err(err) = sender.send(transaction).await {
                                        eprintln!("failed to send transaction to channel: {err}");
                                    };
                                }

                                None => eprintln!("need tx to send transaction"),
                            }
                        }
                    }

                    // TODO: validate block & check if its already in the chain
                    NetMessage::NewBlock(block) => {
                        {
                            // add new block to its chain
                            let mut w_chain = chain.write().await;
                            w_chain.add_blk(&block).unwrap();
                        }

                        // broadcast to its peers
                        Self::broadcast_block(&block, &peers).await;

                        println!();
                    }

                    _ => eprintln!("msg ignored"), // TODO: implement display for msg to specify
                                                   // the ignored msg type
                }
            }
        })
    }
}
