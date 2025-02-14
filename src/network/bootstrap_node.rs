#![allow(dead_code)]

use super::Node;

/// this node will start the network, gen genesis block, start the chain
/// say, entry point of this project
const BOOTSTRAP_PORT: u16 = 8333;

#[derive(Debug)]
pub struct BootstrapNode {
    node: Node,
}

impl BootstrapNode {
    pub fn new() -> Self {
        Self {
            node: Node::new(BOOTSTRAP_PORT),
        }
    }

    pub async fn run(&self) {
        // TODO: below
        // 1. create genesis block
        // 2. start the chain
        // 3. listen to peers -> this will be blocking the thread
        // 4. if getaddr -> send peers back
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    #[test]
    fn new() {
        let bnode = BootstrapNode::new();
        dbg!(bnode);
    }
}
