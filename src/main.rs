mod blockchain;
mod network;
mod node;
mod utils;
mod wallet;

use core::time;
use std::error::Error;
use std::net::SocketAddr;

use tokio::task::JoinSet;

use self::node::{BootstrapNode, FullNode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut js = JoinSet::new();

    js.spawn(async move {
        let bdnode = BootstrapNode::new(8001);
        bdnode.run().await;
    });

    std::thread::sleep(time::Duration::from_secs(3));

    js.spawn(async move {
        let addr: SocketAddr = "127.0.0.1:8001".parse().unwrap();
        let fullnode = FullNode::new(addr, 8002).await;
        fullnode.run().await;
    });

    js.spawn(async move {
        let addr: SocketAddr = "127.0.0.1:8001".parse().unwrap();
        let fullnode = FullNode::new(addr, 8003).await;
        fullnode.run().await;
    });

    js.join_all().await;

    Ok(())
}
