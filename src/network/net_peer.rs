use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct Peer {
    uuid: String,
    socket_addr: String,
    stream: Arc<Mutex<TcpStream>>,
}

impl Peer {
    pub fn get_uuid(&self) -> &String {
        &self.uuid
    }
}
