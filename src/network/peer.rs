#![allow(dead_code)]

use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Peer {
    uuid: String,
    socket_addr: SocketAddr,
    stream: Mutex<TcpStream>,
}

impl Peer {
    pub fn new(uuid: &str, socket_addr: SocketAddr, stream: TcpStream) -> Self {
        Self {
            uuid: uuid.to_string(),
            socket_addr,
            stream: Mutex::new(stream),
        }
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        // 1. check uuid
        if self.uuid != other.uuid {
            return false;
        }

        // 2. socket addr
        if self.socket_addr != other.socket_addr {
            return false;
        }

        true
    }
}
