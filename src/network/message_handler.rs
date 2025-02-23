#![allow(dead_code)]

use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::blockchain::{Block, Transaction};
use crate::network::errors::{NetworkError, NetworkResult};
use crate::network::network_operations::NetOps;

use serde::{Deserialize, Serialize};

use super::network_node::NetworkNodeInfo;

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
pub struct NetworkNodeMessageHandler;

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
}
