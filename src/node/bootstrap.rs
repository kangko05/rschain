#![allow(dead_code)]

use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::Chain;
use crate::network::{
    MessageHandler, NetOps, NetworkMessage, NetworkNode, NetworkNodeMessageHandler, NetworkResult,
};

/*
 * bootstrap node
 */

#[derive(Debug)]
pub struct BootstrapNode {
    network_node: NetworkNode,
    chain: Arc<RwLock<Chain>>,
    msg_handler: BootstrapMessageHandler,
}

impl BootstrapNode {
    pub fn new(port: u16) -> Self {
        let chain = Arc::new(RwLock::new(Chain::new()));

        Self {
            network_node: NetworkNode::new(port),
            msg_handler: BootstrapMessageHandler::new(Arc::clone(&chain)),
            chain,
        }
    }

    pub async fn run(&self) {
        let socket_addr = self.network_node.get_addr();
        let msg_handler = self.msg_handler.clone();
        let (tx, mut rx) = mpsc::channel(100);

        let listen_handle =
            tokio::spawn(async move { NetOps::listen(socket_addr, msg_handler, tx).await });

        let node = Arc::new(RwLock::new(self.network_node.clone()));
        let channel_handle = tokio::spawn(async move {
            while let Some((msg, tx)) = rx.recv().await {
                let node = Arc::clone(&node);
                match msg {
                    NetworkMessage::FindNode { target_id } => {
                        NetworkNode::handle_find_node(node, tx, &target_id).await;
                    }

                    NetworkMessage::AddNode { id, addr } => {
                        NetworkNode::handle_add_node(node, &id, addr).await;
                    }

                    _ => {}
                }

                println!();
            }
        });

        tokio::select! {
            listen_result = listen_handle => {
                if let Err(err) = listen_result {
                    eprintln!("failed to join listen handle: {err}");
                }
            }

            channel_result = channel_handle => {
                if let Err(err) = channel_result {
                    eprintln!("failed to join channel handle: {err}");
                }
            }
        }
    }
}

/*
 * bootstrap node message handler
 */

#[derive(Debug, Clone)]
struct BootstrapMessageHandler {
    base: NetworkNodeMessageHandler,
    chain: Arc<RwLock<Chain>>,
}

#[async_trait]
impl MessageHandler for BootstrapMessageHandler {
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
                self.base.handle_add_node(req_tx, id, *addr).await?
            }
            NetworkMessage::GetChain => self.handle_get_chain(stream).await?,

            _ => {}
        }

        Ok(())
    }
}

impl BootstrapMessageHandler {
    pub fn new(chain: Arc<RwLock<Chain>>) -> Self {
        Self {
            base: NetworkNodeMessageHandler {},
            chain,
        }
    }

    async fn handle_get_chain(&self, stream: &mut TcpStream) -> NetworkResult<()> {
        println!("got get chain message");

        let blocks = self.chain.read().await.get_blocks().to_vec();
        dbg!(&blocks);

        let response = NetworkMessage::Blocks(blocks);
        NetOps::write(stream, response).await?;

        Ok(())
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    #[test]
    fn new() {
        let btnode = BootstrapNode::new(8000);
        dbg!(btnode);
    }
}
