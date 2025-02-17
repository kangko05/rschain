#![allow(dead_code)]

use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use super::errors::NetResult;
use super::{FullNode, NetMessage, NetOps, NodeOps, NodeType, RunNode};

use crate::blockchain::{Block, Transaction, TxPool};

const MAX_POOL: usize = 15;

/// full node + mining functions
pub struct MiningNode {
    node: FullNode,
    mempool: Arc<RwLock<TxPool>>,
}

// constructor
impl MiningNode {
    pub async fn init(port: u16) -> NetResult<Self> {
        Ok(Self {
            node: FullNode::init(port).await?,
            mempool: Arc::new(RwLock::new(TxPool::new())),
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
            Some(tx_tx),
        );
        let mine_handle = self.handle_transaction(tx_rx);

        let _ = tokio::join!(listen_handle, msg_handle, mine_handle);

        Ok(())
    }
}

// miner operations
impl MiningNode {
    pub fn handle_transaction(&self, mut rx: mpsc::Receiver<Transaction>) -> JoinHandle<()> {
        let pool = Arc::clone(&self.mempool);
        let chain = Arc::clone(self.node.get_chain());
        let peers = Arc::clone(self.node.get_peers());
        tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                let pool_size = pool.read().await.len();
                if pool_size == MAX_POOL - 1 {
                    // mine block
                    // broadcast to network
                    let (prev_hash, block_height) = {
                        let r_chain = chain.read().await;
                        let prev_hash = r_chain.get_last_block_hash_string().unwrap();
                        let block_height = r_chain.get_block_height();

                        (prev_hash, block_height)
                    };

                    let blk = {
                        // mine block;
                        let mut txs = vec![Transaction::coinbase("", block_height).unwrap()];
                        let mut w_pool = pool.write().await;

                        while w_pool.len() > 0 && txs.len() < MAX_POOL {
                            txs.push(w_pool.get_one().unwrap());
                        }

                        let mut blk = Block::new(&prev_hash, &txs).unwrap();

                        blk.mine().unwrap();

                        blk
                    };

                    // broadcast block
                    NodeOps::broadcast_block(&blk, &peers).await;

                    // add to chain
                    {
                        let mut w_chain = chain.write().await;
                        w_chain.add_blk(&blk).unwrap();
                    }
                } else {
                    let mut w_pool = pool.write().await;
                    w_pool.add_one(&transaction);
                }
            }

            println!();
        })
    }
}
