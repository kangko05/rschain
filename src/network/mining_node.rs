#![allow(dead_code)]

use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use super::errors::NetResult;
use super::{FullNode, NetMessage, NetOps, NodeOps, NodeType, RunNode};

use crate::blockchain::{Block, Transaction, TxPool};

const MAX_POOL: usize = 3;

/// full node + mining functions
pub struct MiningNode {
    node: FullNode,
    miner_addr: String,
}

// constructor
impl MiningNode {
    // TODO: think about this
    // - getting address as a parameter for now -> can't change it while node is running
    // - could receive address through another channel but it might complicate things more
    pub async fn init(port: u16, miner_addr: &str) -> NetResult<Self> {
        Ok(Self {
            node: FullNode::init(port).await?,
            miner_addr: miner_addr.to_string(),
        })
    }
}

impl RunNode for MiningNode {
    async fn run(&self) -> NetResult<()> {
        let (tx, rx) = mpsc::channel::<(NetMessage, TcpStream)>(32);
        let (tx_tx, tx_rx) = mpsc::channel::<Transaction>(32);

        let listen_handle = NetOps::listen(self.node.get_addr(), tx);
        let msg_handle = NodeOps::handle_messages(
            NodeType::Mining,
            self.node.get_peers(),
            self.node.get_chain(),
            rx,
            self.node.get_mempool(),
            Some(tx_tx),
        );

        let mine_handle = self.handle_transaction(tx_rx);

        let _ = tokio::join!(listen_handle, msg_handle, mine_handle);

        Ok(())
    }
}

// miner operations
impl MiningNode {
    // TODO: currently its logging and continuing if error occurs, think about implementing below stuffs
    // 1. retry
    // 2. throwing failed transactions away
    // 3. monitoring critical errors
    pub fn handle_transaction(&self, mut rx: mpsc::Receiver<Transaction>) -> JoinHandle<()> {
        let pool = Arc::clone(self.node.get_mempool());
        let chain = Arc::clone(self.node.get_chain());
        let peers = Arc::clone(self.node.get_peers());
        let miner_addr = self.miner_addr.to_string();
        tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                // for testing
                let tx_id = String::from_utf8_lossy(transaction.get_id());
                println!("got tx: {tx_id}");

                let pool_size = pool.read().await.len();
                if pool_size == MAX_POOL - 1 {
                    println!("got enough tx, start mining...");

                    // mine block
                    // broadcast to network
                    let (prev_hash, block_height) = {
                        let r_chain = chain.read().await;
                        let Ok(prev_hash) = r_chain.get_last_block_hash_string() else {
                            eprintln!("failed to get previous hash");
                            continue;
                        };
                        let block_height = r_chain.get_block_height();

                        (prev_hash, block_height)
                    };

                    let blk = {
                        // mine block;
                        let Ok(coinbase) = Transaction::coinbase(&miner_addr, block_height) else {
                            eprintln!("failed to create coinbase transaction");
                            continue;
                        };

                        let mut txs = vec![coinbase];
                        let mut w_pool = pool.write().await;

                        while let Some(tx) = w_pool.get_one() {
                            txs.push(tx);
                        }

                        let Ok(mut blk) = Block::new(&prev_hash, &txs) else {
                            eprintln!("failed to create new block");
                            continue;
                        };

                        if let Err(err) = blk.mine() {
                            eprintln!("failed to mine the new block: {err}");
                            continue;
                        };

                        blk
                    };

                    // broadcast block
                    NodeOps::broadcast_block(&blk, &peers).await;

                    // add to chain
                    {
                        let mut w_chain = chain.write().await;
                        if let Err(err) = w_chain.add_blk(&blk) {
                            eprintln!("failed to add block to chain: {err}");
                            continue;
                        };
                    }
                } else {
                    let mut w_pool = pool.write().await;
                    w_pool.add_one(&transaction);
                }
            }
        })
    }
}
