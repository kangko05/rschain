#![allow(dead_code)]

use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use super::errors::NetworkError;
use super::errors::NetworkResult;
use super::network_operations::NetOps;

use serde::{Deserialize, Serialize};

use super::node::NodeInfo;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    FindNode { target_id: Vec<u8> },
    FoundNode { nodes: Vec<NodeInfo> },

    Ping,
    Pong,

    AddNode { id: Vec<u8>, addr: SocketAddr },
    Ok,
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
pub struct NodeMessageHandler;

#[async_trait]
impl MessageHandler for NodeMessageHandler {
    async fn handle_message(
        &self,
        stream: &mut TcpStream,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        msg: &NetworkMessage,
    ) -> NetworkResult<()> {
        match msg {
            NetworkMessage::Ping => Self::handle_ping(stream).await?,
            NetworkMessage::FindNode { target_id } => {
                Self::handle_find_node(stream, req_tx, target_id).await?
            }
            NetworkMessage::AddNode { id, addr } => {
                Self::handle_add_node(req_tx, id, *addr).await?
            }

            _ => {}
        }

        Ok(())
    }
}

impl NodeMessageHandler {
    async fn handle_ping(stream: &mut TcpStream) -> NetworkResult<()> {
        NetOps::write(stream, NetworkMessage::Pong).await
    }

    async fn handle_find_node(
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
    async fn handle_add_node(
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>,
        target_id: &[u8],
        socket_addr: SocketAddr,
    ) -> NetworkResult<()> {
        let msg = NetworkMessage::AddNode {
            id: target_id.to_vec(),
            addr: socket_addr,
        };

        let (tx, _) = oneshot::channel();

        if let Err(err) = req_tx.send((msg, tx)).await {
            return Err(NetworkError::Str(format!("{err}")));
        };

        Ok(())
    }
}
