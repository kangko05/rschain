#![allow(dead_code)]

use std::net::SocketAddr;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::network::NetMessage;

use super::NetOps;

const BOOTSTRAP_PORT: u16 = 8000;

#[derive(Debug)]
pub struct BootstrapNode {
    uuid: String,
    socket_addr: SocketAddr,
}

impl BootstrapNode {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            socket_addr: NetOps::get_socket_addr(BOOTSTRAP_PORT),
        }
    }
}

// runner
impl BootstrapNode {
    // listen -> channel -> handle message
    pub async fn run(&self) {
        let mut join_set = tokio::task::JoinSet::<()>::new();

        let (tx, rx) = mpsc::channel::<NetMessage>(32);

        // 1. listen to requests
        join_set.spawn(async move {
            let _ = NetOps::listen(BOOTSTRAP_PORT, tx).await;
        });

        // 2. handle message

        join_set.spawn(async move {
            Self::handle_message(rx).await;
        });

        println!("running bootstrap node");

        join_set.join_all().await;
    }
}

// handle message
impl BootstrapNode {
    async fn handle_message(mut rx: mpsc::Receiver<NetMessage>) {
        while let Some(msg) = rx.recv().await {
            dbg!(msg);
        }
    }
}

#[cfg(test)]
mod bootstrap_test {
    use super::*;

    #[test]
    fn new() {
        let bn = BootstrapNode::new();
        dbg!(bn);
    }
}
