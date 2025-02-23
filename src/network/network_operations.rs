#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use super::errors::{NetworkError, NetworkResult};
use super::message_handler::{MessageHandler, NetworkMessage};
use super::network_node::NetworkNodeInfo;

#[derive(Debug)]
pub struct NetOps;

impl NetOps {
    /// send msg, msg len first, then actual msg
    pub async fn write(stream: &mut TcpStream, net_msg: NetworkMessage) -> NetworkResult<()> {
        let payload = serde_json::to_vec(&net_msg)?;
        let len = (payload.len() as u32).to_be_bytes();

        let _ = stream.write(&len).await?;
        let _ = stream.write(&payload).await?;

        Ok(())
    }

    /// read msg len first, then payload
    /// parse payload into network message before returning it
    pub async fn read(stream: &mut TcpStream) -> NetworkResult<NetworkMessage> {
        let mut len_buf = [0u8; 4]; // expectin be bytes of u32
        if stream.read_exact(&mut len_buf).await? == 0 {
            return Err(NetworkError::ConnectionClosed);
        };

        let len = u32::from_be_bytes(len_buf);
        let mut payload_buf = vec![0u8; len as usize];
        stream.read_exact(&mut payload_buf).await?;

        Ok(serde_json::from_slice::<NetworkMessage>(&payload_buf)?)
    }

    pub async fn ping(node_info: &NetworkNodeInfo) -> NetworkResult<bool> {
        let mut stream = TcpStream::connect(node_info.get_addr()).await?;

        NetOps::write(&mut stream, NetworkMessage::Ping).await?;

        match tokio::time::timeout(Duration::from_secs(5), NetOps::read(&mut stream)).await {
            Ok(Ok(NetworkMessage::Pong)) => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn listen(
        socket_addr: SocketAddr,
        msg_handler: impl MessageHandler + 'static,
        req_tx: mpsc::Sender<(NetworkMessage, oneshot::Sender<NetworkMessage>)>, // for communication between handler & node
    ) -> NetworkResult<()> {
        let listener = TcpListener::bind(socket_addr).await?;
        println!("listening to: {socket_addr}");

        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    println!("connection from {addr}");

                    let handler = msg_handler.clone();
                    let req_tx = req_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            let req_tx = req_tx.clone();
                            match Self::read(&mut stream).await {
                                Ok(msg) => {
                                    if let Err(err) =
                                        handler.handle_message(&mut stream, req_tx, &msg).await
                                    {
                                        eprintln!("failed to send response: {err}");
                                        break;
                                    };
                                }

                                Err(err) => {
                                    if err.to_string().contains("early eof")
                                        || err.to_string().contains("connection reset")
                                    {
                                        // connection closed normally
                                        eprintln!("connection closed");
                                    } else {
                                        // unexpected
                                        eprintln!("failed to read message from connection: {err}");
                                    }

                                    break;
                                }
                            };
                        }
                    });
                }
                Err(err) => eprintln!("failed to accept connection: {err}"),
            }
        }
    }

    /// returns failed broadcast list
    /// retry n times
    pub async fn broadcast(
        close_nodes: &[NetworkNodeInfo],
        msg: NetworkMessage,
    ) -> Vec<NetworkNodeInfo> {
        let mut failed = vec![];

        for node in close_nodes {
            match TcpStream::connect(node.get_addr()).await {
                Ok(mut stream) => {
                    if let Err(err) = Self::write(&mut stream, msg.clone()).await {
                        eprintln!("failed to broadcast to {}: {}", node.get_addr(), err);
                        failed.push(node.clone());
                        continue;
                    };
                }

                Err(err) => {
                    eprintln!("failed to broadcast to {}: {}", node.get_addr(), err);
                    failed.push(node.clone());
                    continue;
                }
            }
        }

        failed
    }
}
