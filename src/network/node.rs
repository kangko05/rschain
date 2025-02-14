#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use uuid::Uuid;

use crate::utils;

use super::errors::NetworkResult;

#[derive(Debug)]
pub struct Node {
    uuid: Uuid,
    ip_addr: IpAddr,
    port: u16,
    socket_string: String,
    start_time: u64,
    peers: Arc<Mutex<HashMap<String, bool>>>,
}

impl Node {
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let socket_string = format!("{}:{}", ip_addr, 8333);

        Self {
            uuid,
            ip_addr,
            port: 8333,
            start_time: utils::unixtime_now().unwrap(),
            socket_string,
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn listen(&self) -> NetworkResult<()> {
        let listener = TcpListener::bind(&self.socket_string).await?;
        println!("listening to port {}", self.port);

        let _ = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, _)) => println!("connected from {}", socket.peer_addr().unwrap()),
                    Err(err) => eprintln!("{}", err),
                }
            }
        })
        .await;

        Ok(())
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
}
