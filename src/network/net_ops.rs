#![allow(dead_code)]

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use super::errors::NetResult;
use super::NetMessage;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time;

use colored::Colorize;
use serde::Serialize;

/// handles basic network operations - reading stream, writing stream, ...
pub struct NetOps;

impl NetOps {
    // basic listener -> channel -> handle message
    pub async fn listen(port: u16, tx: mpsc::Sender<NetMessage>) -> NetResult<()> {
        let socket_addr = Self::get_socket_addr(port);
        let listener = TcpListener::bind(socket_addr).await?;

        let mut handles = vec![];

        while let Ok((stream, addr)) = listener.accept().await {
            let tx = tx.clone();
            let handle = tokio::spawn(async move {
                let _ = time::timeout(
                    time::Duration::from_secs(3),
                    Self::read_stream(stream, addr, tx),
                )
                .await;
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Err(err) = handle.await {
                eprintln!("join error: {err}");
            };
        }

        Ok(())
    }

    // using locahost for this project
    pub fn get_socket_addr(port: u16) -> SocketAddr {
        let localhost = IpAddr::V4(Ipv4Addr::LOCALHOST);
        format!("{localhost}:{port}")
            .parse()
            .expect("shouldn't fail parsing into socket addr...")
    }

    async fn read_stream(mut stream: TcpStream, addr: SocketAddr, tx: mpsc::Sender<NetMessage>) {
        loop {
            let mut len_buf = [0u8; 4];

            if let Err(err) = stream.read_exact(&mut len_buf).await {
                if Self::is_connection_reset(&err) {
                    eprintln!("disconnected ({}): {err}", addr);
                    break;
                } else {
                    eprintln!("invalid message length ({}): {err}", addr);
                    continue;
                }
            };

            let len = u32::from_be_bytes(len_buf);
            let mut buf = vec![0u8; len as usize];

            if let Err(err) = stream.read_exact(&mut buf).await {
                if Self::is_connection_reset(&err) {
                    eprintln!("disconnected ({}): {err}", addr);
                    break;
                } else {
                    eprintln!("invalid message length ({}): {err}", addr);
                    continue;
                }
            }

            match serde_json::from_slice::<NetMessage>(&buf) {
                Ok(msg) => {
                    if let Err(err) = tx.send(msg).await {
                        eprintln!("failed to forward message to handler: {err}");
                        continue;
                    };
                }

                Err(err) => {
                    eprintln!("failed to parse msg bytes");
                    continue;
                }
            };
        }
    }

    fn is_connection_reset(err: &io::Error) -> bool {
        err.kind() == io::ErrorKind::UnexpectedEof || err.kind() == io::ErrorKind::ConnectionReset
    }

    pub async fn send_message(to: &str, msg: impl Serialize) -> NetResult<()> {
        let mut stream = TcpStream::connect(to).await?;

        let msg = serde_json::to_vec(&msg)?;
        let msg_len = (msg.len() as u32).to_be_bytes();

        // send length first
        stream.write_all(&msg_len).await?;
        stream.write_all(&msg).await?;

        Ok(())
    }
}
