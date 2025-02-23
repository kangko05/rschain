#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::{Transaction, TxPool};
use crate::network::{MessageHandler, NetOps, NetworkMessage, NetworkNode, NetworkResult};

use super::full::FullMessageHandler;
use super::FullNode;

/*
 * mining node
 */

pub struct MiningNode {
    fullnode: FullNode,
    msg_handler: MiningMessageHandler,
    mempool: Arc<RwLock<TxPool>>,
}

impl MiningNode {
    pub async fn new(bootstrap_addr: SocketAddr, port: u16) -> Self {
        let node = FullNode::new(bootstrap_addr, port).await;
        let base_handler = node.get_msg_handler().clone();

        //let node = Arc::new(RwLock::new(node));
        let mempool = Arc::new(RwLock::new(TxPool::new()));

        let msg_handler = MiningMessageHandler::new(base_handler, Arc::clone(&mempool)).await;

        Self {
            fullnode: node,
            mempool,
            msg_handler,
        }
    }

    pub async fn run(&self) {
        let node = self.fullnode.get_base_node();
        let msg_handler = self.msg_handler.clone();
        let listen_handle = tokio::spawn(async move { NetworkNode::run(node, msg_handler).await });

        // wait for listener to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // broadcast to other nodes
        let (id, addr, close_nodes) = {
            let base_node = self.fullnode.get_base_node();
            let r_node = base_node.read().await;

            let id = r_node.get_id().to_vec();
            let addr = r_node.get_addr();

            let close_nodes = r_node.find_node(&id, 20);

            (id, addr, close_nodes)
        };

        let mut req_nodes = close_nodes;
        let newnode_msg = NetworkMessage::NewNode { id, addr };

        // retry broadcasting
        for _ in 0..3 {
            let failed = NetOps::broadcast(&req_nodes, newnode_msg.clone()).await;

            if failed.is_empty() {
                break;
            } else {
                req_nodes = failed;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        if let Err(err) = listen_handle.await {
            eprintln!("failed to join listen handle: {err}");
        };
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
}
