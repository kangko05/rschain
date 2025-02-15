#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::utils;

use super::errors::{NetworkError, NetworkResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    GetAddr,
    GetAddrResp(Vec<String>),
    Ping,
    Pong,

    GetChain,
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

type PeerType = HashMap<String, TcpStream>;
type PeersMapType = Arc<Mutex<PeerType>>;
pub type ChanMessageType = (Vec<u8>, String, TcpStream);

pub trait NodeOperation {
    async fn run(&mut self) -> NetworkResult<()>;
}

#[derive(Debug)]
/// contains basic opertions for nodes
/// - listen, ping, connect to other peers
pub struct Node {
    uuid: Uuid,
    ip_addr: IpAddr,
    port: u16,
    socket_string: String,
    start_time: u64,
    peers: PeersMapType,
    pong_resps: Arc<Mutex<Vec<String>>>, // stores pong responses from peer
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
            pong_resps: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn test_run(&mut self) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel::<ChanMessageType>(32);

        let listen_handle = self.listen(tx)?;
        let message_handle = self.handle_basic_messages(rx, None);
        let ping_handle = self.ping();

        println!("listening to {}", self.port);

        match tokio::try_join!(listen_handle, message_handle, ping_handle) {
            Ok(_) => {}
            Err(err) => eprintln!("{err}"),
        };

        Ok(())
    }

    pub fn listen(&mut self, tx: mpsc::Sender<ChanMessageType>) -> NetworkResult<JoinHandle<()>> {
        let node_addr = self.socket_string.clone();

        Ok(tokio::spawn(async move {
            let listener = if let Ok(listener) = TcpListener::bind(&node_addr).await {
                listener
            } else {
                return;
            };

            loop {
                match listener.accept().await {
                    Ok((stream, socket_addr)) => {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            match Self::read_stream(stream).await {
                                Ok((buf, stream)) => {
                                    let addr = socket_addr.to_string();

                                    // handle msg with buf
                                    if let Err(err) = tx.send((buf, addr, stream)).await {
                                        eprintln!("Failed to send message to handler: {}", err);
                                    };
                                }

                                Err(err) => eprintln!("{}", err),
                            };
                        });
                    }

                    Err(err) => eprintln!("{err}"),
                }
            }
        }))
    }

    /// this only handles basic network messages - getaddr, ping, and pong
    /// to handle more messages, pass in another tx so this method can send the msg to that
    /// channel
    /// also, adding to peers map should be handled in extended handler
    pub fn handle_basic_messages(
        &self,
        mut rx: mpsc::Receiver<ChanMessageType>,
        ext_tx: Option<mpsc::Sender<(NetworkMessage, String, TcpStream)>>,
    ) -> JoinHandle<()> {
        let peers = Arc::clone(&self.peers);
        let pong_resps = Arc::clone(&self.pong_resps);

        tokio::spawn(async move {
            while let Some((msg, addr, stream)) = rx.recv().await {
                let net_msg = match serde_json::from_slice::<NetworkMessage>(&msg) {
                    Ok(msg) => msg,
                    Err(err) => {
                        eprintln!("failed to parse the message: {}", err);
                        continue;
                    }
                };

                let processed =
                    Self::process_message(net_msg.clone(), &peers, &pong_resps, &addr).await;

                let Some(processed) = processed else {
                    eprintln!("message processing failed");
                    continue;
                };

                if processed.is_empty() {
                    if let Some(tx) = &ext_tx {
                        if let Err(err) = tx.send((net_msg, addr.to_string(), stream)).await {
                            eprintln!("failed to forward message to extended handler: {}", err);
                        }
                    } else {
                        eprintln!("received message for extended handler but no handle available ");
                    }
                } else {
                    let stream = match Self::write_response(processed, stream).await {
                        Ok(stream) => stream,
                        Err(err) => {
                            eprintln!("failed to send the response: {}", err);
                            continue;
                        }
                    };

                    Self::add_peers(&peers, &addr, stream).await;

                    dbg!(&peers);
                }
            }
        })
    }

    pub async fn connect_to_peer(&mut self, addr: String) -> NetworkResult<()> {
        let socket = TcpSocket::new_v4()?;
        socket.set_keepalive(true)?;

        let stream = socket.connect(addr.parse()?).await?;
        let addr = stream.peer_addr()?.to_string();
        let mut peers_guard = self.peers.lock().await;
        peers_guard.insert(addr, stream);

        Ok(())
    }

    pub fn ping(&self) -> JoinHandle<()> {
        let peers = Arc::clone(&self.peers);
        let pong_resps = Arc::clone(&self.pong_resps);

        tokio::spawn(async move {
            loop {
                if let Ok(ping_msg) = serde_json::to_vec(&json!(NetworkMessage::Ping)) {
                    let mut peers_guard = peers.lock().await;
                    let mut pong_resps_guard = pong_resps.lock().await;
                    let msg_length = (ping_msg.len() as u32).to_be_bytes();

                    if !pong_resps_guard.is_empty() {
                        peers_guard.retain(|addr, _| pong_resps_guard.contains(addr));
                    }

                    for (_, stream) in peers_guard.iter_mut() {
                        let _ = stream.write_all(&msg_length).await;
                        let _ = stream.write_all(&ping_msg).await;
                    }

                    pong_resps_guard.clear();
                };

                sleep(Duration::from_secs(60 * 15)); // 15 min cycle
            }
        })
    }

    pub fn get_peers_map(&self) -> &PeersMapType {
        &self.peers
    }
}

// helper functions
impl Node {
    pub async fn add_peers(peers: &PeersMapType, addr: &str, stream: TcpStream) {
        let mut peers_guard = peers.lock().await;
        peers_guard.insert(addr.to_string(), stream);
    }

    pub async fn write_response(msg: Vec<u8>, mut stream: TcpStream) -> NetworkResult<TcpStream> {
        // get length
        let msg_len = (msg.len() as u32).to_be_bytes();

        stream.write_all(&msg_len).await?;
        stream.write_all(&msg).await?;

        Ok(stream)
    }

    /// read stream -> &[u8]
    async fn read_stream(stream: TcpStream) -> NetworkResult<(Vec<u8>, TcpStream)> {
        let mut length_buf = [0u8; 4];
        let mut stream = stream;

        if let Ok(n) = stream.read_exact(&mut length_buf).await {
            if n == 0 {
                // TODO
                // disconnected
                // try reconnecting? or leave it as it is
            }

            let length = u32::from_be_bytes(length_buf);
            let mut buf = vec![0u8; length as usize];

            match stream.read_exact(&mut buf).await {
                Ok(_) => return Ok((buf, stream)),
                Err(err) => return Err(err.into()),
            };
        }

        Err(NetworkError::str("failed to read length bytes"))
    }

    async fn remove_peer(peers: &mut Arc<Mutex<HashMap<String, TcpStream>>>, addr: &str) {
        let mut peers_guard = peers.lock().await;
        peers_guard.remove(addr);
    }

    async fn process_message(
        msg: NetworkMessage,
        peers: &PeersMapType,
        pong_resps: &Arc<Mutex<Vec<String>>>,
        addr: &str,
    ) -> Option<Vec<u8>> {
        match msg {
            NetworkMessage::GetAddr => {
                println!("get addr!");
                return Self::getaddr_resp_json(peers).await.ok();
            }

            NetworkMessage::Ping => {
                println!("ping!");
                return serde_json::to_vec(&json!(NetworkMessage::Pong)).ok();
            }

            NetworkMessage::Pong => {
                println!("pong!");
                let mut pong_resps_guard = pong_resps.lock().await;
                pong_resps_guard.push(addr.to_string());
                return Some(vec![]);
            }

            _ => println!("ignoring for now"),
        }

        None
    }

    async fn getaddr_resp_json(peers: &Arc<Mutex<PeerType>>) -> NetworkResult<Vec<u8>> {
        let peers_guard = peers.lock().await;
        let resp = peers_guard
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<String>>();

        Ok(serde_json::to_vec(&json!(resp))?)
    }
}

// apparently this can't handle async closure because its not stable?
//fn obtain_mutex_lock<T, U, F>(mutex: Arc<Mutex<T>>, func: F) -> NetworkResult<U>
//where
//    F: FnOnce(MutexGuard<T>) -> NetworkResult<U>,
//{
//    match mutex.lock() {
//        Ok(mutex_guard) => func(mutex_guard),
//        Err(err) => Err(NetworkError::str(&err.to_string())),
//    }
//}
//
//#[cfg(test)]
//mod node_tests {
//    use super::*;
//
//    #[tokio::test]
//    async fn new() {
//        let mut node = Node::new(8000);
//        let _ = node.test_run().await;
//        dbg!(node);
//    }
//}
