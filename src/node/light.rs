#![allow(dead_code)]

use core::time;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::blockchain::BlockHeader;
use crate::network::{
    MessageHandler, NetOps, NetworkMessage, NetworkNode, NetworkNodeMessageHandler, NetworkResult,
};

/*
 * Light Node
 */
pub struct LightNode {
    network_node: Arc<RwLock<NetworkNode>>,
    headers: Vec<BlockHeader>,
    msg_handler: LightMessageHandler,
}

impl LightNode {
    /// seed addrs can be address one of [bootstrap, full, mining] node
    pub async fn new(seed_addrs: &[SocketAddr], port: u16) -> Self {
        let node = Arc::new(RwLock::new(NetworkNode::new(port)));
        let blk_headers = Self::get_chain_data(seed_addrs)
            .await
            .expect("failed to get chain data from seed nodes");

        Self {
            msg_handler: LightMessageHandler::new(Arc::clone(&node)).await,
            network_node: node,
            headers: blk_headers,
        }
    }

    async fn get_chain_data(seed_addrs: &[SocketAddr]) -> NetworkResult<Vec<BlockHeader>> {
        let mut curr_blocks = vec![];

        for addr in seed_addrs {
            let mut stream = TcpStream::connect(addr).await?;

            NetOps::write(&mut stream, NetworkMessage::GetChain).await?;
            match NetOps::read(&mut stream).await {
                Ok(NetworkMessage::Blocks(blks)) => {
                    if blks.len() > curr_blocks.len() {
                        for blk in blks {
                            curr_blocks.push(blk.get_header().clone());
                        }
                    }
                }

                Err(err) => {
                    eprintln!("failed to get chain data from {}: {}", addr, err);
                    continue;
                }

                _ => {
                    eprintln!("failed to get chain data from {}", addr);
                    continue;
                }
            }
        }

        Ok(curr_blocks)
    }

    pub async fn run(&self) {
        let network_node = Arc::clone(&self.network_node);
        let msg_handler = self.msg_handler.clone();
        let listen_handle = tokio::spawn(async move {
            NetworkNode::run(network_node, msg_handler).await;
        });

        tokio::time::sleep(time::Duration::from_millis(100)).await;

        NetOps::broadcast_self(Arc::clone(&self.network_node)).await;

        let _ = tokio::join!(listen_handle);
    }
}

/*
 * Light Node Message Handler
 */

#[derive(Debug, Clone)]
struct LightMessageHandler {
    base: NetworkNodeMessageHandler,
}

#[async_trait]
impl MessageHandler for LightMessageHandler {
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

            NetworkMessage::NewTx(transaction) => {
                self.base.handle_new_tx(stream, transaction).await?
            }

            //NetworkMessage::NewBlock(new_blk) => self.handle_new_block(new_blk.clone()).await?,
            _ => {}
        }

        Ok(())
    }
}

impl LightMessageHandler {
    pub async fn new(network_node: Arc<RwLock<NetworkNode>>) -> Self {
        Self {
            base: NetworkNodeMessageHandler::new(network_node).await,
        }
    }
}
