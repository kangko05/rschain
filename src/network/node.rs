#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::utils;

use super::errors::{NetworkError, NetworkResult};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    GetAddr,
    GetAddrResp(Vec<String>),
    Ping,
    Pong,
}

impl NetworkMessage {
    pub fn create_getaddr_resp(peers_map: HashMap<String, TcpStream>) -> Self {
        let keys_vec = peers_map
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<String>>();

        Self::GetAddrResp(keys_vec)
    }
}

#[derive(Debug)]
pub struct Node {
    uuid: Uuid,
    ip_addr: IpAddr,
    port: u16,
    socket_string: String,
    start_time: u64,
    peers: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
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
        println!("listening to {}", self.port);

        // ping

        match tokio::try_join!(listen_handle,) {
            Ok(_) => {}
            Err(err) => eprintln!("{err}"),
        };

        Ok(())
    }

    fn listen(&mut self) -> NetworkResult<JoinHandle<()>> {
        let node_addr = self.socket_string.clone();
        let peers = Arc::clone(&self.peers);

        Ok(tokio::spawn(async move {
            let listener = if let Ok(listener) = TcpListener::bind(&node_addr).await {
                listener
            } else {
                return;
            };

            loop {
                match listener.accept().await {
                    Ok((stream, socket_addr)) => {
                        let peers = Arc::clone(&peers);
                        tokio::spawn(async move {
                            Self::read_stream(stream, &socket_addr.to_string(), peers).await;
                        });
                    }

                    Err(err) => eprintln!("{err}"),
                }
            }
        }))
    }

    pub async fn connect_to_peer(&mut self, addr: String) -> NetworkResult<()> {
        let socket = TcpSocket::new_v4()?;
        socket.set_keepalive(true)?;

        let stream = socket.connect(addr.parse()?).await?;
        let addr = stream.peer_addr()?.to_string();
        if let Ok(mut peers_guard) = self.peers.lock() {
            peers_guard.insert(addr, Arc::new(stream));
        }

        Ok(())
    }
}

// helper functions
impl Node {
    async fn read_stream(
        stream: TcpStream,
        addr: &str,
        peers: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
    ) {
        let mut buf = [0; 1024];
        let mut stream = stream;
        loop {
            match stream.read(&mut buf).await {
                // client disconnected
                // - just log disconnection
                // - removing peers will be handled through ping method
                Ok(0) => {
                    println!("{addr} disconnectd");
                    break;
                }

                // handle message here
                // - planning to communicate through json format
                Ok(n) => {
                    if let Some(resp) = Self::handle_message(&buf[..n], Arc::clone(&peers)) {
                        stream.write_all(&resp).await.unwrap();
                    }
                }

                Err(err) => eprintln!("{err}"),
            };
        }

        // add peer
        match peers.lock() {
            Ok(mut peers_guard) => {
                peers_guard.insert(addr.to_string(), Arc::new(stream));
            }
            Err(err) => eprintln!("failed to add peer {addr}: {err}"),
        }

        dbg!(&peers);
    }

    fn remove_peer(peers: &mut Arc<Mutex<HashMap<String, TcpStream>>>, addr: &str) {
        match peers.lock() {
            Ok(mut peers_guard) => {
                if peers_guard.remove(addr).is_none() {
                    eprintln!("failed to remove {addr}");
                }
            }

            Err(err) => eprintln!("failed to remove {addr}: {err}"),
        };
    }
    fn handle_message(
        msg_buf: &[u8],
        peers: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
    ) -> Option<Vec<u8>> {
        if let Ok(msg) = serde_json::from_slice::<NetworkMessage>(msg_buf) {
            match msg {
                NetworkMessage::GetAddr => {
                    return obtain_mutex_lock(Arc::clone(&peers), Self::getaddr_resp_json).ok();
                }

                NetworkMessage::Ping => {
                    return serde_json::to_vec(&json!(NetworkMessage::Pong)).ok();
                }

                _ => println!("ignoring for now"),
            }
        } else {
            let msg = String::from_utf8_lossy(msg_buf);
            println!("{msg}");
        }

        None
    }

    fn getaddr_resp_json(
        peers: MutexGuard<HashMap<String, Arc<TcpStream>>>,
    ) -> NetworkResult<Vec<u8>> {
        let resp = peers
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<String>>();

        Ok(serde_json::to_vec(&json!(resp))?)
    }
}

// apparently this can't handle async closure because its not stable?
fn obtain_mutex_lock<T, U, F>(mutex: Arc<Mutex<T>>, func: F) -> NetworkResult<U>
where
    F: FnOnce(MutexGuard<T>) -> NetworkResult<U>,
{
    match mutex.lock() {
        Ok(mutex_guard) => func(mutex_guard),
        Err(err) => Err(NetworkError::str(&err.to_string())),
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
}
