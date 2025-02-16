#![allow(dead_code)]

use std::sync::Arc;

use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    block::{Block, BlockResult, Chain, Transaction},
    network::node::ChanMessageType,
};

use super::{errors::NetworkResult, node::NodeOperation, NetworkMessage, Node};

/// this node will start the network, gen genesis block, start the chain
/// say, entry point of this project
/// this node will only listen, not making peer discovery actions
const BOOTSTRAP_PORT: u16 = 8333;

#[derive(Debug)]
pub struct BootstrapNode {
    node: Node,
    chain: Chain,
}

impl BootstrapNode {
    pub fn new() -> Self {
        let blockchain = Self::init_chain()
            .expect("failing to initializing a blockchain MUST panic -  so panicking");

        Self {
            node: Node::new(BOOTSTRAP_PORT),
            chain: blockchain,
        }
    }

    fn init_chain() -> BlockResult<Chain> {
        let tx = Transaction::coinbase("genesis", 0)?;
        let blk = Block::new("", &vec![tx])?;
        let mut chain = Chain::new();

        chain.genesis(&blk)?;

        Ok(chain)
    }
}

impl NodeOperation for BootstrapNode {
    async fn run(&mut self) -> NetworkResult<()> {
        println!("running bootstrap node");

        let (tx, rx) = mpsc::channel::<ChanMessageType>(32);
        let (ext_tx, ext_rx) = mpsc::channel(32);

        let base_listener_handle = self.node.listen(tx)?;
        let base_msghandler_handle = self.node.handle_basic_messages(rx, Some(ext_tx));
        let msghandler_handle = self.handle_message(ext_rx);
        let ping_handle = self.node.ping();

        match tokio::try_join!(
            base_listener_handle,
            base_msghandler_handle,
            msghandler_handle,
            ping_handle
        ) {
            Ok(_) => {}
            Err(err) => eprintln!("{err}"),
        };

        Ok(())
    }
}

impl BootstrapNode {
    fn handle_message(
        &self,
        mut rx: mpsc::Receiver<(NetworkMessage, String, TcpStream)>,
    ) -> JoinHandle<()> {
        let peers = Arc::clone(self.node.get_peers_map());
        let blocks = self.chain.get_blocks().clone();

        tokio::spawn(async move {
            while let Some((msg, addr, stream)) = rx.recv().await {
                let Some(processed) = Self::process_message(msg, &blocks) else {
                    eprintln!("failed to process message",);
                    continue;
                };

                let stream = match Node::write_message(processed, stream).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        eprintln!("failed to send response: {}", err);
                        continue;
                    }
                };

                Node::add_peers(&peers, &addr, stream).await;
            }
        })
    }

    /// process NetworkMessage ->  Vec<u8>
    fn process_message(msg: NetworkMessage, blocks: &Vec<Block>) -> Option<Vec<u8>> {
        match msg {
            NetworkMessage::GetChain => {
                let blocks = blocks.clone();
                return serde_json::to_vec(&json!(blocks))
                    .map_err(|err| eprintln!("failed to serialize block: {}", err))
                    .ok();
            }
            _ => None,
        }
    }
}
