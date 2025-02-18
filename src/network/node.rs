#![allow(dead_code)]

/*
 * basic node
 */

use std::net::SocketAddr;
use std::sync::Arc;

use uuid::Uuid;

use super::{utils, Peer};

pub struct Node {
    uuid: String,
    socket_addr: SocketAddr,
    peers: Arc<Vec<Peer>>,
}

impl Node {
    pub fn new(port: u16) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            socket_addr: utils::get_socket_addr(port),
            peers: Arc::new(Vec::new()),
        }
    }
}
