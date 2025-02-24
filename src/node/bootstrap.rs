#![allow(dead_code)]

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::blockchain::{Block, BlockResult, Chain, Transaction};
use crate::network::NetworkNode;
use crate::utils;

use super::full::FullMessageHandler;

/*
 * bootstrap node
 * - initialize first chain with genesis block
 */

#[derive(Debug)]
pub struct BootstrapNode {
    network_node: Arc<RwLock<NetworkNode>>,
    chain: Arc<RwLock<Chain>>,
    msg_handler: FullMessageHandler,
}

impl BootstrapNode {
    pub async fn new(port: u16) -> Self {
        // bootstrap node must have chain
        let chain = Self::init_chain().expect("failed to init chain in bootstrap node");
        let chain = Arc::new(RwLock::new(chain));

        let network_node = NetworkNode::new(port);
        let id = network_node.get_id().to_vec();
        let addr = network_node.get_addr();

        let network_node = Arc::new(RwLock::new(network_node));

        Self {
            msg_handler: FullMessageHandler::new(
                Arc::clone(&chain),
                Arc::clone(&network_node),
                &id,
                addr,
            )
            .await,
            network_node,
            chain,
        }
    }

    fn init_chain() -> BlockResult<Chain> {
        let mut chain = Chain::new();

        let genesis_hash = utils::hash_to_string(&utils::sha256("genesis".as_bytes()));
        let first_tx = Transaction::coinbase(&genesis_hash, chain.get_block_height())?;

        let mut blk = Block::new(&genesis_hash, &[first_tx])?;

        blk.mine()?;
        chain.genesis(&blk)?;

        Ok(chain)
    }

    pub async fn run(&self) {
        let node = Arc::clone(&self.network_node);
        let handler = self.msg_handler.clone();
        let listen_handle =
            tokio::spawn(async move { NetworkNode::run(node, handler).await }).await;

        if let Err(err) = listen_handle {
            eprintln!("failed to join listen handle: {err}");
        }
    }
}
