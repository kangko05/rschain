#![allow(dead_code)]

use std::collections::HashMap;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

#[derive(Serialize, Deserialize, Clone)]
pub struct Peer {
    ip_addr: IpAddr,
    port: u16,
    uuid: String,
}

impl Peer {
    pub fn get_socket_addr(&self) -> String {
        format!("{}:{}", self.ip_addr, self.port)
    }

    pub fn get_uuid(&self) -> String {
        self.uuid.to_string()
    }
}

// map
pub struct PeersMap {
    peers_map: HashMap<String, (Peer, TcpStream)>,
}

impl PeersMap {
    pub fn new() -> Self {
        Self {
            peers_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, peer: Peer, stream: TcpStream) {
        self.peers_map.insert(peer.get_uuid(), (peer, stream));
    }

    pub fn remove(&mut self, uuid: &str) {
        self.peers_map.remove(uuid);
    }

    pub fn get_addr_vec(&self) -> Vec<String> {
        self.peers_map
            .iter()
            .map(|(_, (peer, _))| peer.get_socket_addr())
            .collect::<Vec<String>>()
    }
}
