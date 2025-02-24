#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::{Block, BlockResult, Chain, Transaction, TxPool};
use crate::network::{MessageHandler, NetOps, NetworkMessage, NetworkNode, NetworkResult};
use crate::wallet::Wallet;

use super::full::FullMessageHandler;
use super::{FullNode, MAX_RETRY, NUM_CLOSE_NODES};

/*
 * mining node
 */

pub struct MiningNode {
    fullnode: FullNode,
    msg_handler: MiningMessageHandler,
    mempool: Arc<RwLock<TxPool>>,
    miner_wallet: Wallet, // in real blockchain system, they have mining pool for mining
                          // maybe I can add it later (TODO)
}

impl MiningNode {
    pub async fn new(bootstrap_addr: SocketAddr, port: u16, miner_wallet: Wallet) -> Self {
        let node = FullNode::new(bootstrap_addr, port).await;
        let base_handler = node.get_msg_handler().clone();

        let mempool = Arc::new(RwLock::new(TxPool::new()));

        let msg_handler = MiningMessageHandler::new(base_handler, Arc::clone(&mempool)).await;

        Self {
            fullnode: node,
            mempool,
            msg_handler,
            miner_wallet,
        }
    }

    pub async fn run(&self) {
        let node = self.fullnode.get_base_node();
        let msg_handler = self.msg_handler.clone();
        let listen_handle = tokio::spawn(async move { NetworkNode::run(node, msg_handler).await });

        // wait for listener to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // watch tx pool
        let chain = self.fullnode.get_chain();
        let mempool = Arc::clone(&self.mempool);
        let basenode = self.fullnode.get_base_node();
        let addr = self.miner_wallet.get_address().clone();
        tokio::spawn(async move {
            loop {
                if mempool.read().await.len() == 14 {
                    let chain = Arc::clone(&chain);
                    let mempool = Arc::clone(&mempool);

                    match Self::mine(&addr, chain.clone(), mempool).await {
                        Ok(blk) => {
                            let r_node = basenode.read().await;
                            let id = r_node.get_id();
                            let close_nodes = r_node.find_node(id, NUM_CLOSE_NODES);
                            NetOps::broadcast(&close_nodes, NetworkMessage::NewBlock(blk.clone()))
                                .await;

                            if let Err(err) = chain.write().await.add_blk(&blk) {
                                eprintln!("failed to add block to the chain: {err}");
                                // TODO: get chain data again maybe?
                            };
                        }

                        Err(err) => {
                            eprintln!("failed to mine new block: {err}");
                            continue;
                        }
                    };
                }
            }
        });

        // broadcast to other nodes
        NetOps::broadcast_self(Arc::clone(&self.fullnode.get_base_node())).await;

        if let Err(err) = listen_handle.await {
            eprintln!("failed to join listen handle: {err}");
        };
    }

    pub async fn mine(
        coinbase_to: &str,
        chain: Arc<RwLock<Chain>>,
        mempool: Arc<RwLock<TxPool>>,
    ) -> BlockResult<Block> {
        let block_height = chain.read().await.get_block_height();

        let coinbase = Transaction::coinbase(coinbase_to, block_height)?;
        let mut txs = vec![coinbase];
        let mempool_tx = mempool.write().await.clear();

        for tx in mempool_tx {
            txs.push(tx);
        }

        let prev_hash = chain.read().await.get_last_block_hash_string()?;
        let mut blk = Block::new(&prev_hash, &txs)?;
        blk.mine()?;

        Ok(blk)
    }
}

/*
 * mining node message handler
 */

#[derive(Debug, Clone)]
struct MiningMessageHandler {
    base: FullMessageHandler,
    mempool: Arc<RwLock<TxPool>>,
    //chain: Arc<RwLock<Chain>>,
    //id: Vec<u8>,
    //addr: SocketAddr,
}

#[async_trait]
impl MessageHandler for MiningMessageHandler {
    async fn handle_message(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        msg: &NetworkMessage,
    ) -> NetworkResult<()> {
        match msg {
            NetworkMessage::Ping => self.base.handle_ping(stream).await?,

            NetworkMessage::FindNode { target_id } => {
                self.base
                    .handle_find_node(stream, req_tx, target_id)
                    .await?
            }

            NetworkMessage::AddNode { id, addr } => {
                self.base.handle_add_node(stream, req_tx, id, *addr).await?
            }

            NetworkMessage::GetChain => self.base.handle_get_chain(stream).await?,

            NetworkMessage::NewTx(new_tx) => self.handle_new_tx(stream, new_tx).await?,

            NetworkMessage::NewBlock(new_blk) => self.handle_new_block(new_blk.clone()).await?,

            _ => {}
        }

        Ok(())
    }
}

impl MiningMessageHandler {
    pub async fn new(base_handler: FullMessageHandler, mempool: Arc<RwLock<TxPool>>) -> Self {
        Self {
            base: base_handler,
            mempool,
        }
    }

    pub async fn handle_new_tx(
        &self,
        stream: &mut TcpStream,
        new_tx: &Transaction,
    ) -> NetworkResult<()> {
        // TODO: need to verify new tx
        self.mempool.write().await.add_one(new_tx);
        self.base.handle_new_tx(stream, new_tx).await
    }

    pub async fn handle_new_block(&self, block: Block) -> NetworkResult<()> {
        self.base.handle_new_block(block.clone()).await?; // verify, broadcast, and add to chain here

        let txs = block.get_transactions()?;
        self.mempool.write().await.remove_transactions(txs);

        Ok(())
    }
}
