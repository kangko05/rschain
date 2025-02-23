#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::TxPool;
use crate::network::{MessageHandler, NetworkMessage, NetworkResult};

use super::full::FullMessageHandler;
use super::FullNode;

/*
 * mining node
 */

pub struct MiningNode {
    fullnode: Arc<RwLock<FullNode>>,
    mempool: TxPool,
}

impl MiningNode {
    pub async fn new(bootstrap_addr: SocketAddr, port: u16) -> Self {
        let node = FullNode::new(bootstrap_addr, port).await;
        let node = Arc::new(RwLock::new(node));

        Self {
            fullnode: node,
            mempool: TxPool::new(),
        }
    }
}

/*
 * mining node message handler
 */

#[derive(Debug, Clone)]
struct MiningMessageHandler {
    fullnode: Arc<RwLock<FullNode>>,
    base: FullMessageHandler,
}

#[async_trait]
impl MessageHandler for MiningMessageHandler {
    async fn handle_message(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        msg: &NetworkMessage,
    ) -> NetworkResult<()> {
        Ok(())
    }
}

impl MiningMessageHandler {
    pub async fn new(fullnode: Arc<RwLock<FullNode>>) -> Self {
        let base = fullnode.read().await.get_msg_handler().clone();
        Self { base, fullnode }
    }
}
