#![allow(dead_code)]

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::{Block, Transaction};
use crate::network::errors::{NetworkError, NetworkResult};
use crate::network::network_operations::NetOps;

use serde::{Deserialize, Serialize};

use super::network_node::NetworkNodeInfo;
use super::NetworkNode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    Ok,

    GetChain,
    Blocks(Vec<Block>), // response for 'GetChain'

    Ping,
    Pong,

    NewTx(Transaction), // broadcast

    // need to send tx to node
    NewNode { id: Vec<u8>, addr: SocketAddr }, // this need to be broadcasted
    AddNode { id: Vec<u8>, addr: SocketAddr }, // this will add nodes

    FindNode { target_id: Vec<u8> },
    FoundNode { nodes: Vec<NetworkNodeInfo> },

    NewBlock(Block),
}

#[async_trait]
pub trait MessageHandler: Send + Sync + Clone {
    async fn handle_message(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        msg: &NetworkMessage,
    ) -> NetworkResult<()>;
}

#[derive(Clone, Debug)]
pub struct NetworkNodeMessageHandler {
    node: Arc<RwLock<NetworkNode>>,
    id: Vec<u8>,
}

#[async_trait]
impl MessageHandler for NetworkNodeMessageHandler {
    async fn handle_message(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        msg: &NetworkMessage,
    ) -> NetworkResult<()> {
        match msg {
            NetworkMessage::Ping => self.handle_ping(stream).await?,
            NetworkMessage::FindNode { target_id } => {
                self.handle_find_node(stream, req_tx, target_id).await?
            }
            NetworkMessage::AddNode { id, addr } | NetworkMessage::NewNode { id, addr } => {
                self.handle_add_node(stream, req_tx, id, *addr).await?
            }

            _ => {}
        }

        Ok(())
    }
}

impl NetworkNodeMessageHandler {
    pub async fn new(node: Arc<RwLock<NetworkNode>>) -> Self {
        let id = node.read().await.get_id().clone();
        Self { node, id }
    }

    pub async fn handle_ping(&self, stream: &mut TcpStream) -> NetworkResult<()> {
        NetOps::write(stream, NetworkMessage::Pong).await
    }

    pub async fn handle_find_node(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        target_id: &[u8],
    ) -> NetworkResult<()> {
        let msg = NetworkMessage::FindNode {
            target_id: target_id.to_vec(),
        };

        let (tx, rx) = oneshot::channel();

        if let Err(err) = req_tx.send((msg, tx)).await {
            return Err(NetworkError::Str(format!("{err}")));
        };

        match rx.await {
            Ok(msg) => Ok(NetOps::write(stream, msg).await?),
            Err(err) => Err(NetworkError::Str(format!("{err}"))),
        }
    }

    // run add_node in node
    // TODO: think about this - maybe send Ok back to client? - do i even need to receive from rx
    // here?
    pub async fn handle_add_node(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        target_id: &[u8],
        socket_addr: SocketAddr,
    ) -> NetworkResult<()> {
        let msg = NetworkMessage::AddNode {
            id: target_id.to_vec(),
            addr: socket_addr,
        };

        let (tx, rx) = oneshot::channel();

        if let Err(err) = req_tx.send((msg, tx)).await {
            return Err(NetworkError::Str(format!("{err}")));
        };

        match rx.await {
            Ok(_) => Ok(NetOps::write(stream, NetworkMessage::Ok).await?),
            Err(err) => Err(NetworkError::Str(format!("{err}"))),
        }
    }

    // broadcast new tx
    pub async fn handle_new_tx(
        &self,
        stream: &mut TcpStream,
        new_tx: &Transaction,
    ) -> NetworkResult<()> {
        // TODO: think about how to verify new transaction

        let msg = NetworkMessage::NewTx(new_tx.clone());
        let close_nodes = self.node.read().await.find_node(&self.id, 20);
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
