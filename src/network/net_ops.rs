#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::network::errors::NetError;

use super::errors::NetResult;

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    GetPeers,
}

/// Network Operations for nodes
/// - setup listener -> channel -> parse -> handle meesage in node
/// - messages will contain length+payload
pub struct NetOps;

impl NetOps {
    pub fn listen(addr: &str, tx: mpsc::Sender<NetMessage>) -> JoinHandle<()> {
        let node_addr = addr.to_string();

        tokio::spawn(async move {
            let listener = match TcpListener::bind(&node_addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    eprintln!("Failed to bind: {err}");
                    return;
                }
            };

            Self::handle_connections(listener, tx).await;
        })
    }

    async fn handle_connections(listener: TcpListener, tx: mpsc::Sender<NetMessage>) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    Self::spawn_connection_handler(stream, tx.clone()).await;
                }
                Err(err) => eprintln!("Accept error: {err}"),
            }
        }
    }

    async fn spawn_connection_handler(stream: TcpStream, tx: mpsc::Sender<NetMessage>) {
        match Self::read_stream(stream).await {
            Ok(buf) => {
                let msg = serde_json::from_slice::<NetMessage>(&buf).unwrap();

                if let Err(err) = tx.send(msg).await {
                    eprintln!("Failed to send message to handler: {err}");
                }
            }
            Err(err) => eprintln!("Stream read error: {err}"),
        }
    }

    async fn read_stream(mut stream: TcpStream) -> NetResult<Vec<u8>> {
        // read length ->  read payload
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_ok() {
            let length = u32::from_be_bytes(len_buf);
            let mut payload_buf = vec![0u8; length as usize];

            stream.read_exact(&mut payload_buf).await?;

            Ok(payload_buf)
        } else {
            Err(NetError::str("failed to read msg length"))
        }
    }
}

#[cfg(test)]
mod netops_test {
    use serde_json::json;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc;

    use super::*;

    async fn send_msg() {
        let msg = serde_json::to_vec(&json!(NetMessage::GetPeers)).unwrap();
        let msg_len = (msg.len() as u32).to_be_bytes();

        let mut stream = TcpStream::connect("127.0.0.1:8000")
            .await
            .expect("failed to connect");

        stream
            .write_all(&msg_len)
            .await
            .expect("failed to write msg len");

        stream
            .write_all(&msg)
            .await
            .expect("failed to write msg bytes");
    }

    #[tokio::test]
    async fn listen() {
        let (tx, mut rx) = mpsc::channel(32);

        let listen_handle = NetOps::listen("127.0.0.1:8000", tx);

        println!("abc");

        let send_handle = tokio::spawn(async move {
            send_msg().await;
        });

        let thandle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // handle messages here
                dbg!(msg);
            }
        });

        let _ = tokio::try_join!(listen_handle, send_handle, thandle);
    }
}
