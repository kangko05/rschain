#![allow(dead_code)]

use std::future::Future;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::network::errors::NetError;

use super::errors::NetResult;

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // req
    GetChain,
    GetPeers,

    // resp
    Peers(Vec<String>), // socket addresses of peers
}

/// Network Operations for nodes
/// - setup listener -> channel -> parse -> handle meesage in node
/// - messages will contain length+payload
/// - get the message through channel
pub struct NetOps;

impl NetOps {}

impl NetOps {
    pub fn abc<F>(mut rx: mpsc::Receiver<(NetMessage, TcpStream)>, handle_msg: F) -> JoinHandle<()>
    where
        F: Fn(NetMessage, TcpStream) + Send + 'static,
    {
        tokio::spawn(async move {
            while let Some((msg, stream)) = rx.recv().await {
                // handle message here
                handle_msg(msg, stream);
            }
        })
    }

    pub async fn write_message(stream: &mut TcpStream, msg: impl Serialize) -> NetResult<()> {
        let msg_bytes = serde_json::to_vec(&msg)?;
        let msg_len = (msg_bytes.len() as u32).to_be_bytes();

        stream.write_all(&msg_len).await?;
        stream.write_all(&msg_bytes).await?;

        Ok(())
    }
}

// listen
impl NetOps {
    pub fn listen(addr: &str, tx: mpsc::Sender<(NetMessage, TcpStream)>) -> JoinHandle<()> {
        let addr = addr.to_string();

        tokio::spawn(async move {
            let listener = TcpListener::bind(addr)
                .await
                .expect("failed to bind address");

            loop {
                let tx = tx.clone();
                match listener.accept().await {
                    Ok((stream, _)) => Self::handle_connection(stream, tx).await,
                    Err(err) => eprintln!("failed to accept connection: {err}"),
                }
            }
        })
    }

    async fn handle_connection(mut stream: TcpStream, tx: mpsc::Sender<(NetMessage, TcpStream)>) {
        tokio::spawn(async move {
            match Self::read_stream(&mut stream).await {
                Ok(buf) => {
                    match serde_json::from_slice::<NetMessage>(&buf) {
                        Ok(msg) => {
                            if let Err(err) = tx.send((msg, stream)).await {
                                eprintln!("failed to send msg to channel: {err}");
                            };
                        }

                        Err(err) => eprintln!("failed to parse message: {err}"),
                    };
                }
                Err(err) => eprintln!("failed to read stream: {err}"),
            }
        });
    }

    async fn read_stream(stream: &mut TcpStream) -> NetResult<Vec<u8>> {
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
