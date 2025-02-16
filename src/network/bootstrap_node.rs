#![allow(dead_code)]

/*
 * BootstrapNode
 * - responsible for initializing & starting the chain
 * - also, this node will be entry point of the network
 *
 * 1. init chain
 *    - create genesis block
 *    - setup initial chain state
 *
 * 2. requests to be handled - responses will be broadcasted to current peers
 *    - get peers -> respond with current peers list
 *    - get chain -> respond with current chain
 *    - new peer connection -> validate & add to peers list
 *    - new block notification -> validate & update chain
 *
 * 3. manage peers
 *    - implement ping/heartbeat
 *    - maintain active peers list
 *    - remove inactive peers
 *    - broadcast important updates to all peers
 *    - handle peer discovery
 */

use std::net::{IpAddr, Ipv4Addr};

use uuid::Uuid;

use crate::blockchain::{Block, Chain, Transaction};

#[derive(Debug)]
pub struct BootstrapNode {
    uuid: String,
    socket_addr: String,
    chain: Chain,
}

const BOOTSTRAP_PORT: u16 = 8000;

impl BootstrapNode {
    pub fn new() -> Self {
        let socket_addr = format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), BOOTSTRAP_PORT);

        Self {
            uuid: Uuid::new_v4().to_string(),
            chain: Self::init_chain(),
            socket_addr,
        }
    }

    // failing to init chain -> panic
    pub fn init_chain() -> Chain {
        let mut chain = Chain::new();

        // first tx
        let tx = Transaction::coinbase("genesis tx", chain.get_block_height())
            .expect("failed to init tx");

        // genesis block
        let mut blk = Block::new("genesis block", &[tx]).expect("failed to init block");
        blk.mine().expect("failed to mine block");

        chain.genesis(&blk).expect("failed to add block to chain");

        chain
    }
}

#[cfg(test)]
mod bootstrap_test {
    use super::*;

    #[test]
    fn new() {
        let boot_node = BootstrapNode::new();
        dbg!(boot_node);
    }
}
