#![allow(dead_code)]

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::{Chain, Transaction};
use crate::network::{
    MessageHandler, NetOps, NetworkError, NetworkMessage, NetworkNode, NetworkNodeMessageHandler,
    NetworkResult,
};

/*
 * full node
 */

#[derive(Debug)]
pub struct FullNode {
    network_node: Arc<RwLock<NetworkNode>>,
    chain: Arc<RwLock<Chain>>,
    msg_handler: FullMessageHandler,
}

// initialize full node
impl FullNode {
    // 1. get chain data
    // 2. inform the network about itself
    // 3. get close nodes info
    // TODO: implementing retry mechanism might be nice
    pub async fn new(bootstrap_addr: SocketAddr, port: u16) -> Self {
        let mut stream = TcpStream::connect(bootstrap_addr)
            .await
            .expect("failed to connect to bootstrap node");

        let chain = Self::get_chain(&mut stream)
            .await
            .expect("failed to chain data from bootstrap node");

        let chain = Arc::new(RwLock::new(chain));

        let mut network_node = NetworkNode::new(port);
        let id = network_node.get_id().to_vec();
        let addr = network_node.get_addr();

        Self::req_add_node(&mut stream, &id, addr)
            .await
            .expect("failed to get request add nodes to bootstrap node");

        Self::get_close_nodes(&mut stream, &id, &mut network_node)
            .await
            .expect("");

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

    pub async fn get_chain(stream: &mut TcpStream) -> NetworkResult<Chain> {
        NetOps::write(stream, NetworkMessage::GetChain).await?;
        let blocks = NetOps::read(stream).await?;

        match blocks {
            NetworkMessage::Blocks(blks) => Ok(Chain::from(blks)?),
            _ => Err(NetworkError::str("failed to receive blocks")),
        }
    }

    pub async fn req_add_node(
        stream: &mut TcpStream,
        id: &[u8],
        addr: SocketAddr,
    ) -> NetworkResult<()> {
        let msg = NetworkMessage::AddNode {
            id: id.to_vec(),
            addr,
        };

        NetOps::write(stream, msg).await?;

        let _ = NetOps::read(stream).await?;

        Ok(())
    }

    pub async fn get_close_nodes(
        stream: &mut TcpStream,
        id: &[u8],
        network_node: &mut NetworkNode,
    ) -> NetworkResult<()> {
        let msg = NetworkMessage::FindNode {
            target_id: id.to_vec(),
        };

        NetOps::write(stream, msg).await?;
        let resp = NetOps::read(stream).await?;

        match resp {
            NetworkMessage::FoundNode { nodes } => {
                for node in nodes {
                    network_node.add_node(node.get_id(), node.get_addr()).await;
                }

                Ok(())
            }

            _ => Err(NetworkError::str("failed to get peer nodes info")),
        }
    }

    pub fn get_msg_handler(&self) -> &FullMessageHandler {
        &self.msg_handler
    }
}

// run full node
impl FullNode {
    pub async fn run(&self) {
        let node = Arc::clone(&self.network_node);
        let handler = self.msg_handler.clone();

        let listen_handle = tokio::spawn(async move {
            NetworkNode::run(node, handler).await;
        });

        // wait for listener to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // broadcast to other nodes
        let id = self.network_node.read().await.get_id().to_vec();
        let addr = self.network_node.read().await.get_addr();

        let close_nodes = self.network_node.read().await.find_node(&id, 20);
        let mut req_nodes = close_nodes;

        let newnode_msg = NetworkMessage::NewNode { id, addr };

        // retry broadcasting
        for _ in 0..3 {
            let failed = NetOps::broadcast(&req_nodes, newnode_msg.clone()).await;

            if failed.is_empty() {
                break;
            } else {
                req_nodes = failed;
            }
        }

        // wait for listener
        if let Err(err) = listen_handle.await {
            eprintln!("failed to join listen handle: {err}");
        };
    }
}

/*
 * full node message handler
 */

#[derive(Debug, Clone)]
pub struct FullMessageHandler {
    base: NetworkNodeMessageHandler,
    chain: Arc<RwLock<Chain>>,
    network_node: Arc<RwLock<NetworkNode>>,
    id: Vec<u8>,
    addr: SocketAddr,
}

#[async_trait]
impl MessageHandler for FullMessageHandler {
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

            NetworkMessage::GetChain => self.handle_get_chain(stream).await?,

            NetworkMessage::NewTx(transaction) => self.handle_new_tx(stream, transaction).await?,

            _ => {}
        }

        Ok(())
    }
}

impl FullMessageHandler {
    pub async fn new(
        chain: Arc<RwLock<Chain>>,
        network_node: Arc<RwLock<NetworkNode>>,
        id: &[u8],
        addr: SocketAddr,
    ) -> Self {
        let msg_handler = network_node.read().await.get_msg_handler().clone();

        Self {
            base: msg_handler,
            chain,
            network_node,
            id: id.to_vec(),
            addr,
        }
    }

    pub async fn handle_get_chain(&self, stream: &mut TcpStream) -> NetworkResult<()> {
        let blocks = self.chain.read().await.get_blocks().to_vec();
        let response = NetworkMessage::Blocks(blocks);
        NetOps::write(stream, response).await
    }

    // broadcast new tx
    pub async fn handle_new_tx(
        &self,
        stream: &mut TcpStream,
        new_tx: &Transaction,
    ) -> NetworkResult<()> {
        // TODO: think about how to verify new transaction

        let msg = NetworkMessage::NewTx(new_tx.clone());
        let close_nodes = self.network_node.read().await.find_node(&self.id, 20);
        let mut req_nodes = close_nodes;

        for _ in 0..3 {
            let failed = NetOps::broadcast(&req_nodes, msg.clone()).await;

            if failed.is_empty() {
                break;
            } else {
                req_nodes = failed;
            }
        }

        NetOps::write(stream, NetworkMessage::Ok).await
    }
}
