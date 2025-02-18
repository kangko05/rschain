#![allow(dead_code)]

/*
 * basic node - light node
 */

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::network::errors::NetResult;
use crate::network::Peer;

pub struct Node {
    uuid: String,
    peers_out: Arc<RwLock<HashMap<String, Peer>>>, // uuid, outbound peers
}

impl Node {
    pub async fn init() -> Self {
        // get peers list from boot strap node
        // connect to peers

        Self::connect_to_bootstrap("127.0.0.1:8000").await;

        Self {
            uuid: Uuid::new_v4().to_string(),
            peers_out: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn connect_to_bootstrap(bootstrap_socket: &str) {
        let socket = TcpSocket::new_v4().unwrap();
        socket.keepalive().unwrap();

        let mut stream = socket
            .connect(bootstrap_socket.parse().unwrap())
            .await
            .unwrap();

        let msg = serde_json::to_vec("give me").unwrap();
        let msg_len = (msg.len() as u32).to_be_bytes();

        stream.write_all(&msg_len).await.unwrap();
        stream.write_all(&msg).await.unwrap();

        // listen
        let mut buf = [0u8; 1024];

        if let Ok(n) = stream.read(&mut buf).await {
            let addr = String::from_utf8_lossy(&buf[..n]);
            println!("{addr}");
        }
    }

    pub async fn connect_to_peer(&self, uuid: &str, peer_addr: SocketAddr) -> NetResult<()> {
        let socket = TcpSocket::new_v4()?;
        socket.keepalive()?;

        let stream = socket.connect(peer_addr).await?;

        let peer = Peer::new(uuid, peer_addr, stream);

        let mut w_peers = self.peers_out.write().await;
        w_peers.insert(uuid.to_string(), peer);

        Ok(())
    }
}
