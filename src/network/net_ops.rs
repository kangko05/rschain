#![allow(dead_code)]

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use crate::network::errors::NetError;

use super::errors::NetResult;

/// Network Operations for nodes
/// - setup listener -> channel -> parse -> handle meesage in node
/// - messages will contain length+payload
pub struct NetOps;

impl NetOps {
    pub async fn listen(addr: &str, tx: mpsc::Sender<Vec<u8>>) -> NetResult<()> {
        let listener = TcpListener::bind(addr).await?;

        let (mut stream, _) = listener.accept().await?;
        let payload = Self::read_stream(&mut stream).await?;

        let msg = String::from_utf8_lossy(&payload);

        println!("received: {msg}");

        Ok(())
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

#[cfg(test)]
mod netops_test {
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc;

    use super::*;

    async fn send_str(msg: &str) {
        let msg_bytes = msg.as_bytes();
        let msg_len = (msg_bytes.len() as u32).to_be_bytes();

        let mut stream = TcpStream::connect("127.0.0.1:8000")
            .await
            .expect("failed to connect");

        stream
            .write_all(&msg_len)
            .await
            .expect("failed to write msg len");

        stream
            .write_all(msg_bytes)
            .await
            .expect("failed to write msg bytes");
    }

    #[tokio::test]
    async fn listen() {
        let (tx, _) = mpsc::channel(32);

        let listen_handle = tokio::spawn(async move {
            NetOps::listen("127.0.0.1:8000", tx)
                .await
                .map_err(|err| eprintln!("{err}"))
                .expect("failed listening");
        });

        let send_handle = tokio::spawn(async move {
            send_str("hello, world!").await;
        });

        let _ = tokio::join!(listen_handle, send_handle);
    }
}
