mod blockchain;
mod network;
mod node;
mod utils;
mod wallet;

use core::time;
use std::error::Error;
use std::net::SocketAddr;

use tokio::task::JoinSet;

use self::node::{BootstrapNode, FullNode, MiningNode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bootstrap_addr: SocketAddr = "127.0.0.1:8001".parse().unwrap();
    let mut js = JoinSet::new();

    js.spawn(async move {
        let bdnode = BootstrapNode::new(8001);
        bdnode.run().await;
    });

    std::thread::sleep(time::Duration::from_secs(3));

    // full nodes
    js.spawn(async move {
        let fullnode = FullNode::new(bootstrap_addr, 8002).await;

        tokio::spawn(async move {
            std::thread::sleep(std::time::Duration::from_secs(10));
        });

        tokio::spawn(async move {});

        fullnode.run().await;
    });

    js.spawn(async move {
        let fullnode = FullNode::new(bootstrap_addr, 8003).await;
        fullnode.run().await;
    });

    js.spawn(async move {
        let _mining_node = MiningNode::new(bootstrap_addr, 8004).await;
    });

    js.join_all().await;

    Ok(())
}
