#![allow(dead_code)]

use crate::utils;

use super::network_node::NetworkNodeInfo;
use super::network_operations::NetOps;
use std::fmt::Display;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Kbucket {
    idx: usize, // idx in node buckets list
    nodes: Vec<NetworkNodeInfo>,
    wait_nodes: Vec<NetworkNodeInfo>,
    k: usize,
}

// TODO: think about implementing 'refresh list'
impl Kbucket {
    pub fn new(idx: usize, k: usize) -> Self {
        Self {
            idx,
            nodes: Vec::with_capacity(k),
            wait_nodes: Vec::new(),
            k,
        }
    }

    pub async fn push(&mut self, node_id: &[u8], socket_addr: SocketAddr) {
        for info in &self.nodes {
            if info.compare_id(node_id) {
                return;
            }
        }

        let node_info = NetworkNodeInfo::new(node_id, socket_addr);

        if self.nodes.len() < self.k {
            self.nodes.push(node_info);
        } else {
            self.handle_full_bucket(&node_info).await;
        }
    }

    async fn handle_full_bucket(&mut self, node_info: &NetworkNodeInfo) {
        let oldest_node = self.nodes[0].clone();

        match NetOps::ping(&oldest_node).await {
            Ok(res) => {
                let node_info = node_info.clone();
                if res {
                    self.wait_nodes.push(node_info.clone());
                } else {
                    self.nodes.remove(0);
                    self.nodes.push(node_info);
                }
            }
            Err(_) => {
                self.nodes.remove(0);
                self.nodes.push(node_info.clone());
            }
        };
    }

    pub fn get_idx(&self) -> usize {
        self.idx
    }

    pub fn get_nodes(&self) -> &Vec<NetworkNodeInfo> {
        &self.nodes
    }
}

impl Display for Kbucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "idx: {}", self.idx)?;

        write!(f, "nodes: [ ")?;
        for node in &self.nodes {
            write!(f, "{} ", utils::hash_to_string(node.get_id()))?;
        }
        write!(f, "]")?;

        write!(f, "wait nodes: [ ")?;
        for node in &self.wait_nodes {
            write!(f, "{} ", utils::hash_to_string(node.get_id()))?;
        }
        writeln!(f, "]")?;

        write!(f, "k: {}", self.k)
    }
}
