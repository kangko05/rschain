#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::errors::NetworkResult;

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {}

pub struct NetOps;

impl NetOps {
    /// send msg, msg len first, then actual msg
    pub async fn write(stream: &mut TcpStream, msg: NetworkMessage) -> NetworkResult<()> {
        let payload = serde_json::to_vec(&msg)?;
        let len = (payload.len() as u32).to_be_bytes();

        let _ = stream.write(&len).await?;
        let _ = stream.write(&payload).await?;

        Ok(())
    }

    /// read msg len first, then payload
    /// parse payload into network message before returning it
    pub async fn read(stream: &mut TcpStream) -> NetworkResult<NetworkMessage> {
        let mut len_buf = [0u8; 4]; // expectin be bytes of u32
        stream.read_exact(&mut len_buf).await?;

        let len = u32::from_be_bytes(len_buf);
        let mut payload_buf = vec![0u8; len as usize];
        stream.read_exact(&mut payload_buf).await?;

        // deserialize

        Ok(serde_json::from_slice::<NetworkMessage>(&payload_buf)?)
    }
}
