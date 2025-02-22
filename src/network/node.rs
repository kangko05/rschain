#![allow(dead_code)]
/*
 * Node
 * - will be used as base node (or light node)
 * - implemented kademlia
 */

use std::collections::HashMap;
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::utils;

use super::errors::{NetworkError, NetworkResult};
use super::kbucket::Kbucket;
use super::message_handler::{NetworkMessage, NodeMessageHandler};
use super::network_operations::NetOps;

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

impl Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "id: {}", utils::hash_to_string(&self.id))?;
        writeln!(f, "last seen: {}", self.last_seen)?;
        write!(f, "addr: {}", self.socket_addr)
    }
}

#[derive(Debug)]
pub struct Node {
    id: Vec<u8>,
    buckets: Vec<Kbucket>, // routing table
    socket_addr: SocketAddr,

    msg_handler: NodeMessageHandler,
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
            msg_handler: NodeMessageHandler {},
        }
    }

    pub fn get_id(&self) -> &Vec<u8> {
        &self.id
    }

    pub fn get_node_info(&self) -> NodeInfo {
        NodeInfo {
            id: self.id.clone(),
            last_seen: utils::unixtime_now(),
            socket_addr: self.socket_addr,
        }
    }
}

// server
impl Node {
    pub async fn run(node: Arc<RwLock<Self>>) {
        let (tx, mut rx) = mpsc::channel::<(NetworkMessage, oneshot::Sender<NetworkMessage>)>(100);

        // start listener
        let socket_addr = { node.read().await.socket_addr };
        let handler = { node.read().await.msg_handler.clone() };
        let tx = tx.clone();
        let listener_handle =
            tokio::spawn(async move { NetOps::listen(socket_addr, handler, tx).await });

        // handle request from the msg handler
        let node = Arc::clone(&node);
        let channel_handle = tokio::spawn(async move {
            while let Some((msg, tx)) = rx.recv().await {
                match msg {
                    NetworkMessage::FindNode { target_id } => {
                        let msg = NetworkMessage::FoundNode {
                            nodes: node.read().await.find_node(&target_id, 20),
                        };

                        if let Err(err) = tx.send(msg) {
                            eprintln!("failed to send result: {:?}", err);
                        };
                    }

                    NetworkMessage::AddNode { id, addr } => {
                        let mut w_node = node.write().await;
                        w_node.add_node(&id, addr).await;

                        if w_node.socket_addr.port() == 8001 {
                            println!("{w_node}");
                        }

                        //tx.send(NetworkMessage::Ok);
                    }
                    _ => {}
                }
            }
        });

        tokio::select! {
            listener_result = listener_handle => {
                if let Ok(Err(err)) = listener_result {
                    eprintln!("listener stopped: {err}");
                }
            }

            channel_result = channel_handle => {
                if let Err(err) = channel_result {
                    println!("channel has closed unexpectedly: {err}");
                }
            }
        }
    }
}

// routing methods
impl Node {
    pub async fn add_node(&mut self, other_id: &[u8], socket_addr: SocketAddr) {
        let bucket_idx = self.find_bucket_idx(other_id);

        if !self.bucket_exists(bucket_idx) {
            self.buckets.push(Kbucket::new(bucket_idx, 20));
        }

        self.buckets
            .iter_mut()
            .find(|bucket| bucket.get_idx() == bucket_idx)
            .expect("bucket should be in the list")
            .push(other_id, socket_addr)
            .await;

        // TODO: Optimize sorting - current approach sorts entire bucket list after each addition
        // Could be improved by:
        // 1. Using binary search to find insertion position (self.buckets.binary_search_by_key(...))
        // 2. Implementing lazy sorting (sort only when needed)
        self.buckets.sort_by_key(|bucket| bucket.get_idx());
    }

    fn bucket_exists(&self, bucket_idx: usize) -> bool {
        self.buckets
            .binary_search_by_key(&bucket_idx, |bucket| bucket.get_idx())
            .is_ok()
    }

    /// finds first 1 in distance bits (meaning first bit that was different)
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

    pub async fn bootstrap(&mut self, seed_nodes: &[NodeInfo]) -> NetworkResult<()> {
        let id = self.id.clone();

        for seed in seed_nodes {
            self.add_node(seed.get_id(), seed.get_addr()).await;

            let mut stream = TcpStream::connect(seed.get_addr()).await?;
            let msg = NetworkMessage::AddNode {
                id: id.clone(),
                addr: self.socket_addr,
            };

            NetOps::write(&mut stream, msg).await?;
        }

        self.node_lookup(seed_nodes, &id, 20, true).await;
        Ok(())
    }

    /// find close nodes from itself -> request close nodes to find node
    //  1.find node from itself first
    //  2.if not found, send request to closest nodes
    //  3. k -> max # of result
    //      - set to same size as bucket (normally)
    //      - small k will result in inefficent lookup
    //      - large k will result in wasting network traffic
    //  4. if used in bootstrap, it adds nodes along the way
    //
    //  TODO: further optimization
    //  1. set max concurrent requests for network optimazation
    //  2. save all nodes from response -> sort -> pick nodes to request
    pub async fn node_lookup(
        &mut self,
        start_nodes: &[NodeInfo],
        target_id: &[u8],
        k: usize,
        is_bootstrap: bool, // if true adds found nodes to routing table
    ) -> Option<NodeInfo> {
        //let close_nodes = self.find_node(target_id, k);
        if let Some(found) = start_nodes.iter().find(|node| node.compare_id(target_id)) {
            return Some(found.clone());
        }

        let mut nodes_to_req = start_nodes.to_vec();
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
                if visited.contains_key(node.get_id()) {
                    continue;
                }

                visited.insert(node.get_id().clone(), true);
                added_any = true; // just in case

                let target_id = target_id.to_vec();
                let req_addr = node.get_addr();
                join_set
                    .spawn(async move { Self::request_close_nodes(&target_id, req_addr).await });
            }

            if !added_any {
                break;
            }

            // check results
            let mut found_nodes = vec![];

            while let Some(result) = join_set.join_next().await {
                if let Ok(Ok(NetworkMessage::FoundNode { nodes })) = result {
                    if is_bootstrap {
                        for node in &nodes {
                            self.add_node(&node.id, node.get_addr()).await;
                        }
                    }
                    found_nodes.extend(nodes);
                }
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

    async fn request_close_nodes(
        target_id: &[u8],
        req_addr: SocketAddr,
    ) -> NetworkResult<NetworkMessage> {
        let Ok(mut stream) = TcpStream::connect(req_addr).await else {
            return Err(NetworkError::str("close node not found"));
        };

        NetOps::write(
            &mut stream,
            NetworkMessage::FindNode {
                target_id: target_id.to_vec(),
            },
        )
        .await?;

        NetOps::read(&mut stream).await
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

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "id: {}", utils::hash_to_string(&self.id))?;

        if self.buckets.is_empty() {
            write!(f, "buckets: [")?;
        } else {
            writeln!(f, "buckets: [")?;
        }
        for bucket in &self.buckets {
            writeln!(f, "{}", bucket)?;
        }

        writeln!(f, "]")?;

        writeln!(f, "addr: {}", self.socket_addr)
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

    #[tokio::test]
    async fn add_node() {
        let mut node = Node::new(8000);

        for _ in 0..100 {
            let r_id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
            node.add_node(&r_id, "127.0.0.1:8080".parse().unwrap())
                .await;
        }

        dbg!(node.buckets);
    }

    #[tokio::test]
    async fn find_node() {
        //let mut node = Node::new(8000);
        //
        //let r_id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
        //let nodes = node.find_node(&r_id, 0);
        //assert_eq!(nodes.len(), 0);
        //
        //for _ in 0..100 {
        //    let r_id_ = utils::sha256(Uuid::new_v4().to_string().as_bytes());
        //    node.add_node(&r_id_, "127.0.0.1:8080".parse().unwrap())
        //        .await;
        //}
        //
        //let nodes = node.find_node(&r_id, 3);
        //assert_eq!(nodes.len(), 3);
    }
}
