#![allow(dead_code)]

use tokio::task::JoinSet;
use uuid::Uuid;

use crate::utils;

use super::errors::NetworkResult;
use super::kbucket::Kbucket;

#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
    id: Vec<u8>,
    last_seen: u64, // UNIX time in sec
}

impl NodeInfo {
    pub fn new(id: &[u8]) -> Self {
        Self {
            id: id.to_vec(),
            last_seen: utils::unixtime_now(),
        }
    }

    pub fn compare_id(&self, other_id: &[u8]) -> bool {
        self.id.eq(other_id)
    }

    pub fn get_id(&self) -> &Vec<u8> {
        &self.id
    }
}

#[derive(Debug)]
pub struct Node {
    id: Vec<u8>,
    buckets: Vec<Kbucket>, // routing table
}

impl Node {
    pub fn new() -> Self {
        let uuid = Uuid::new_v4().to_string();
        let id = utils::sha256(uuid.as_bytes());

        Self {
            id,
            buckets: Vec::with_capacity(256),
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

        // sort by  distance
        all_nodes.sort_by(|a, b| {
            let dist_a = utils::xor_distance_256(a.get_id(), target_id);
            let dist_b = utils::xor_distance_256(b.get_id(), target_id);

            dist_a.cmp(&dist_b)
        });

        all_nodes.truncate(k);

        all_nodes
    }

    pub fn bootstrap(&self, _seed_nodes: &[NodeInfo]) -> NetworkResult<()> {
        Ok(())
    }

    /// find close nodes from itself -> request close nodes to find node
    pub async fn node_lookup(&self, _target_id: &[u8], _k: usize) -> Option<NodeInfo> {
        //let mut close_nodes = self.find_node(target_id, k);
        //if let Some(found) = close_nodes.iter().find(|node| node.compare_id(target_id)) {
        //    return Some(found.clone());
        //}
        //
        //let mut visited = close_nodes;
        //
        //// concurrent requests
        //loop {
        //    let mut join_set = JoinSet::new();
        //
        //    for node in close_nodes {
        //        if visited.contains(&node) {
        //            continue;
        //        }
        //
        //        join_set.spawn(async move {
        //            // TODO: request to find close nodes to the target
        //        });
        //    }
        //
        //    let result = join_set.join_all().await;
        //
        //    if result.is_empty() {
        //        break;
        //    }
        //
        //    if let Some(found) = result.iter().find(|node| node.compare_id(target_id)) {
        //        return Some(found.clone());
        //    } else {
        //        close_nodes = result;
        //    };
        //}

        None
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn new() {
        let node = Node::new();
        dbg!(node);
    }

    #[test]
    fn add_node() {
        let mut node = Node::new();

        for _ in 0..100 {
            let r_id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
            node.add_node(&r_id);
        }

        dbg!(node.buckets);
    }

    #[test]
    fn find_node() {
        let mut node = Node::new();

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
