#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::utils;

use super::errors::{NetworkError, NetworkResult};
use super::kbucket::Kbucket;
use super::network_operations::{NetOps, NetworkMessage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    id: Vec<u8>,
    last_seen: u64, // UNIX time in sec
    socket_addr: SocketAddr,
}

impl NodeInfo {
    pub fn new(id: &[u8], socket_addr: SocketAddr) -> Self {
        Self {
            id: id.to_vec(),
            last_seen: utils::unixtime_now(),
            socket_addr,
        }
    }

    pub fn compare_id(&self, other_id: &[u8]) -> bool {
        self.id.eq(other_id)
    }

    pub fn get_id(&self) -> &Vec<u8> {
        &self.id
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.socket_addr
    }
}

#[derive(Debug)]
pub struct Node {
    id: Vec<u8>,
    buckets: Vec<Kbucket>, // routing table
    socket_addr: SocketAddr,
}

impl Node {
    pub fn new(port: u16) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let id = utils::sha256(uuid.as_bytes());
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let socket_addr = format!("{}:{}", ip_addr, port)
            .parse::<SocketAddr>()
            .expect("parsing string into socket address failed");

        Self {
            id,
            buckets: Vec::with_capacity(256),
            socket_addr,
        }
    }

    pub fn get_id(&self) -> &Vec<u8> {
        &self.id
    }
}

// routing methods
impl Node {
    pub fn add_node(&mut self, other_id: &[u8]) {
        let bucket_idx = self.find_bucket_idx(other_id);

        if !self.bucket_exists(bucket_idx) {
            self.buckets.push(Kbucket::new(bucket_idx, 20));
        }

        self.buckets
            .iter_mut()
            .find(|bucket| bucket.get_idx() == bucket_idx)
            .expect("bucket should be in the list")
            .push(other_id);

        self.buckets.sort_by_key(|bucket| bucket.get_idx());
    }

    fn bucket_exists(&self, bucket_idx: usize) -> bool {
        self.buckets
            .binary_search_by_key(&bucket_idx, |bucket| bucket.get_idx())
            .is_ok()
    }

    fn find_bucket_idx(&self, other_id: &[u8]) -> usize {
        let distance = utils::xor_distance_256(&self.id, other_id);

        for (i, byte) in distance.iter().enumerate() {
            for j in (0..8).rev() {
                if byte & (1 << j) != 0 {
                    // return idx
                    return (i * 8) + (7 - j);
                };
            }
        }

        256 // same distance
    }

    /// returns k nodes that are close
    // 1. get all nodes from bucket
    // 2. sort them by distance (from target id)
    // 3. return k from it
    pub fn find_node(&self, target_id: &[u8], k: usize) -> Vec<NodeInfo> {
        let mut all_nodes = Vec::new();

        for bucket in &self.buckets {
            for node_info in bucket.get_nodes() {
                all_nodes.push(node_info.clone());
            }
        }

        let mut sorted_nodes = Self::sort_nodes_by_distance(&mut all_nodes, target_id);

        sorted_nodes.truncate(k);

        sorted_nodes
    }

    pub fn bootstrap(&self, _seed_nodes: &[NodeInfo]) -> NetworkResult<()> {
        Ok(())
    }

    /// find close nodes from itself -> request close nodes to find node
    //  1.find node from itself first
    //  2.if not found, send request to closest nodes
    pub async fn node_lookup(&self, target_id: &[u8], k: usize) -> Option<NodeInfo> {
        let close_nodes = self.find_node(target_id, k);
        if let Some(found) = close_nodes.iter().find(|node| node.compare_id(target_id)) {
            return Some(found.clone());
        }

        let mut nodes_to_req = close_nodes.clone();
        let mut visited: HashMap<Vec<u8>, bool> = HashMap::new();
        let max_iter = 20;
        let mut iter = 0;

        loop {
            iter += 1;

            if iter > max_iter {
                break;
            }

            let mut join_set = JoinSet::new();
            let mut added_any = false;

            for node in &nodes_to_req {
                if visited.get(node.get_id()).is_some() {
                    continue;
                }

                visited.insert(node.get_id().clone(), true);
                added_any = true; // just in case

                let target_id = target_id.to_vec();
                let req_addr = node.get_addr();
                join_set.spawn(async move {
                    let Ok(mut stream) = TcpStream::connect(req_addr).await else {
                        return Err(NetworkError::str("close node not found"));
                    };

                    if let Err(err) = NetOps::write(
                        &mut stream,
                        NetworkMessage::FindNode {
                            target_id: target_id.to_vec(),
                        },
                    )
                    .await
                    {
                        return Err(err);
                    };

                    NetOps::read(&mut stream).await
                });
            }

            if !added_any {
                break;
            }

            // check results
            let results = join_set.join_next().await;
            let mut found_nodes = vec![];

            for result in results {
                if let Ok(Ok(msg)) = result {
                    match msg {
                        NetworkMessage::FoundNode { nodes } => {
                            found_nodes.extend(nodes);
                        }
                        _ => continue,
                    }
                }
            }

            if found_nodes.is_empty() {
                break;
            }

            let sorted_nodes = Self::sort_nodes_by_distance(&mut found_nodes, target_id);

            if let Some(found) = sorted_nodes.iter().find(|node| node.compare_id(target_id)) {
                return Some(found.clone());
            } else {
                nodes_to_req = sorted_nodes;
                nodes_to_req.truncate(k);
            };
        }

        None
    }

    fn sort_nodes_by_distance(nodes: &mut [NodeInfo], target_id: &[u8]) -> Vec<NodeInfo> {
        // sort by  distance
        nodes.sort_by(|a, b| {
            let dist_a = utils::xor_distance_256(a.get_id(), target_id);
            let dist_b = utils::xor_distance_256(b.get_id(), target_id);

            dist_a.cmp(&dist_b)
        });

        nodes.to_vec()
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn new() {
        let node = Node::new(8000);
        dbg!(node);
    }

    #[test]
    fn add_node() {
        let mut node = Node::new(8000);

        for _ in 0..100 {
            let r_id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
            node.add_node(&r_id);
        }

        dbg!(node.buckets);
    }

    #[test]
    fn find_node() {
        let mut node = Node::new(8000);

        let r_id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
        let nodes = node.find_node(&r_id, 0);
        assert_eq!(nodes.len(), 0);

        for _ in 0..100 {
            let r_id_ = utils::sha256(Uuid::new_v4().to_string().as_bytes());
            node.add_node(&r_id_);
        }

        let nodes = node.find_node(&r_id, 3);
        assert_eq!(nodes.len(), 3);
    }
}
