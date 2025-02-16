#![allow(dead_code, unused)]

use serde_json::json;
use tokio::{net::TcpStream, sync::mpsc};

use crate::block::{Block, Chain};

use super::{NetworkMessage, NetworkResult, Node, NodeOperation};

#[derive(Debug)]
pub struct FullNode {
    node: Node,
    chain: Chain,
    bootstrap_addr: String,
}

impl FullNode {
    pub fn new(port: u16, bootstrap_addr: &str) -> Self {
        let node = Node::new(port);
        let chain = Chain::new();

        Self {
            node,
            chain,
            bootstrap_addr: bootstrap_addr.to_string(),
        }
    }

    async fn init_chain(&mut self) -> NetworkResult<()> {
        let stream = TcpStream::connect(self.bootstrap_addr.clone()).await?;
        let chain_msg = serde_json::to_vec(&json!(NetworkMessage::GetChain))?;
        let stream = Node::write_message(chain_msg, stream).await?;
        let (resp, _) = Node::read_stream(stream).await?;
        let resp = serde_json::from_slice::<Vec<Block>>(&resp)?;

        self.chain = Chain::from(resp).expect("failed to initalize chain");

        Ok(())
    }
}

impl NodeOperation for FullNode {
    // getaddr -> bootstrap node resp -> listener -> msg handler -> dbg for now
    async fn run(&mut self) -> NetworkResult<()> {
        self.node.init_peers(&self.bootstrap_addr).await?;
        self.init_chain().await?;

        println!("initialization success");

        let (tx, rx) = mpsc::channel(32);

        let base_listener_handle = self.node.listen(tx)?;
        let base_msghandler_handle = self.node.handle_basic_messages(rx, None);

        match tokio::try_join!(base_listener_handle, base_msghandler_handle) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{err}");
            }
        }

        Ok(())
    }
}
