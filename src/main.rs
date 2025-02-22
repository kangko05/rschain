mod blockchain;
mod network;
mod node;
mod utils;
mod wallet;

use core::time;
use std::error::Error;
use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio::task::JoinSet;
use uuid::Uuid;

use self::network::{NetOps, NetworkMessage};
use self::node::BootstrapNode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut js = JoinSet::new();

    js.spawn(async move {
        let bdnode = BootstrapNode::new(8001);
        bdnode.run().await;
    });

    js.spawn(async move {
        for i in (0..3).rev() {
            println!("starting request in {}", i + 1);
            std::thread::sleep(time::Duration::from_secs(1));
        }

        let mut stream = TcpStream::connect("127.0.0.1:8001")
            .await
            .expect("Failed to connect to bootstrap node");

        // 1. ping
        NetOps::write(&mut stream, NetworkMessage::Ping)
            .await
            .expect("failed to send ping message");

        let res = NetOps::read(&mut stream)
            .await
            .expect("failed to receive pong message");

        dbg!(&res);

        // 2. add node (random id for now)
        let r_id = Uuid::new_v4().to_string();
        let r_id = utils::sha256(r_id.as_bytes());
        let addr: SocketAddr = "127.0.0.1:9000".parse().expect("failed to parse");

        NetOps::write(
            &mut stream,
            NetworkMessage::AddNode {
                id: r_id.to_vec(),
                addr,
            },
        )
        .await
        .expect("failed to write add node request");

        let res = NetOps::read(&mut stream)
            .await
            .expect("failed to read response");

        dbg!(res);

        // 3. find node
        let r_id = Uuid::new_v4().to_string();
        let r_id = utils::sha256(r_id.as_bytes());

        NetOps::write(
            &mut stream,
            NetworkMessage::FindNode {
                target_id: r_id.to_vec(),
            },
        )
        .await
        .expect("failed to write find node request");

        let res = NetOps::read(&mut stream)
            .await
            .expect("failed to read response");

        dbg!(res);

        println!("??????");

        // 4. get chain
        NetOps::write(&mut stream, NetworkMessage::GetChain)
            .await
            .expect("failed to write get chain message");

        let res = NetOps::read(&mut stream)
            .await
            .expect("failed to read blocks");

        dbg!(&res);

        println!("???");
    });

    js.join_all().await;

    Ok(())
}
