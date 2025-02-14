#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr};

use uuid::Uuid;

use crate::utils;

#[derive(Debug)]
pub struct Node {
    uuid: Uuid,
    ip_addr: IpAddr,
    port: u16,
    start_time: u64,
}

impl Node {
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);

        Self {
            uuid,
            ip_addr,
            port: 8333,
            start_time: utils::unixtime_now().unwrap(),
        }
    }

    pub fn listen(&self) {}
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
