#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::utils;

use super::errors::NetworkResult;

#[derive(Debug, Serialize)]
struct PeersVec(Vec<String>);

impl FromIterator<String> for PeersVec {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[derive(Debug)]
pub struct Node {
    uuid: Uuid,
    ip_addr: IpAddr,
    port: u16,
    socket_string: String,
    start_time: u64,
    peers: Arc<Mutex<HashMap<String, TcpStream>>>,
}

impl Node {
    pub fn new(port: u16) -> Self {
        let uuid = Uuid::new_v4();
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let socket_string = format!("{}:{}", ip_addr, port);

        Self {
            uuid,
            ip_addr,
            port,
            start_time: utils::unixtime_now().unwrap(),
            socket_string,
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn test_run(&mut self) -> NetworkResult<()> {
        let listen_handle = self.listen()?;

        match tokio::try_join!(listen_handle) {
            Ok(_) => {}
            Err(err) => eprintln!("{err}"),
        };

        Ok(())
    }

    fn listen(&mut self) -> NetworkResult<JoinHandle<()>> {
        let node_addr = self.socket_string.clone();
        let peers = Arc::clone(&self.peers);
        println!("listening to {}", self.port);

        Ok(tokio::spawn(async move {
            let listener = TcpListener::bind(&node_addr).await.unwrap();

            'outer: loop {
                match listener.accept().await {
                    Ok((mut stream, sock_addr)) => {
                        let ss = sock_addr.to_string();

                        // handle get addr here
                        // handle message here
                        // for test
                        let mut buf = [0; 1024];
                        while let Ok(n) = stream.read(&mut buf).await {
                            let msg = String::from_utf8_lossy(&buf[..n]).to_string();

                            match msg.as_str() {
                                "getaddr\n" => {
                                    println!("get address?");
                                    if let Ok(peers_guard) = peers.lock() {
                                        let pm = peers_guard
                                            .iter()
                                            .map(|(key, _)| key.to_string())
                                            .collect::<PeersVec>();

                                        dbg!(pm);
                                    }
                                }

                                "print\n" => println!("msg"),
                                _ => break 'outer,
                            }
                        }

                        match peers.lock() {
                            Ok(mut peers_guard) => {
                                if peers_guard.get(&ss).is_none() {
                                    peers_guard.insert(ss, stream);
                                }
                            }
                            Err(err) => eprintln!("{err}"),
                        }
                    }
                    Err(err) => eprintln!("{err}"),
                };
            }
        }))
    }

    pub async fn connect_to_peer(&mut self, addr: String) -> NetworkResult<()> {
        let socket = TcpSocket::new_v4()?;
        socket.set_keepalive(true)?;

        let stream = socket.connect(addr.parse()?).await?;
        let addr = stream.peer_addr()?.to_string();
        if let Ok(mut peers_guard) = self.peers.lock() {
            peers_guard.insert(addr, stream);
        }

        Ok(())
    }

    /// send pings to peers -> if fail to connect, remove it from peers
    pub async fn ping(&self) {}
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn new() {
        let node = Node::new(8000);
        dbg!(node);
    }
}
