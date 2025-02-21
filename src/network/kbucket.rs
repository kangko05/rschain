#![allow(dead_code)]

use super::node::NodeInfo;

#[derive(Debug)]
pub struct Kbucket {
    idx: usize, // idx in node buckets list
    nodes: Vec<NodeInfo>,
    wait_nodes: Vec<NodeInfo>,
    k: usize,
}

impl Kbucket {
    pub fn new(idx: usize, k: usize) -> Self {
        Self {
            idx,
            nodes: Vec::with_capacity(k),
            wait_nodes: Vec::new(),
            k,
        }
    }

    pub fn push(&mut self, node_id: &[u8]) {
        for info in &self.nodes {
            if info.compare_id(node_id) {
                return;
            }
        }

        let node_info = NodeInfo::new(node_id);

        if self.nodes.len() < self.k {
            self.nodes.push(node_info);
        } else {
            // TODO
            // ping the oldest node (in idx 0)
            // if no response -> remove it and add new one
            self.wait_nodes.push(node_info);
        }
    }

    pub fn get_idx(&self) -> usize {
        self.idx
    }

    pub fn get_nodes(&self) -> &Vec<NodeInfo> {
        &self.nodes
    }
}
