#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::errors::{NetworkError, NetworkResult};
use super::node::NodeInfo;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    FindNode { target_id: Vec<u8> },
    FoundNode { nodes: Vec<NodeInfo> },

    Ping,
    Pong,
}

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

    pub async fn ping(node_info: &NodeInfo) -> NetworkResult<bool> {
        let mut stream = TcpStream::connect(node_info.get_addr()).await?;

        NetOps::write(&mut stream, NetworkMessage::Ping).await?;

        match tokio::time::timeout(Duration::from_secs(5), NetOps::read(&mut stream)).await {
            Ok(Ok(NetworkMessage::Pong)) => Ok(true),
            _ => Ok(false),
        }
    }
}
