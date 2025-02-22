//#![allow(dead_code, unused)]

use core::time;
use std::error::Error;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use self::network::{NetOps, NetworkMessage, Node};

mod blockchain;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut join_set = JoinSet::new();
    let node_count = 3;
    let base_port = 8001;
    let (ready_tx, mut ready_rx) = mpsc::channel::<(u16, Vec<u8>)>(node_count);

    for i in 0..node_count {
        let port: u16 = base_port + (i as u16);
        let ready_tx = ready_tx.clone();

        join_set.spawn(async move {
            let mut node = Node::new(port);
        });
    }

    join_set.join_all().await;

    Ok(())
}

// test 1 - ping & find node
//#[tokio::main]
//async fn main() -> Result<(), Box<dyn Error>> {
//    let mut join_set = JoinSet::new();
//
//    let mut r = rand::thread_rng();
//    let ru: u16 = r.gen_range(0..1000);
//    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
//    join_set.spawn(async move {
//        let mut node1 = Node::new(8001);
//
//        for i in 0..10000 {
//            let new_node = Node::new(i + 8002);
//            let info = new_node.get_node_info();
//
//            node1.add_node(info.get_id(), info.get_addr()).await;
//
//            if ru == i {
//                tx.send(info.get_id().to_vec())
//                    .await
//                    .expect("failed to send random id");
//            }
//        }
//
//        println!("{}", node1);
//
//        let node1 = Arc::new(node1);
//        node1.run().await;
//    });
//
//    join_set.spawn(async move {
//        for i in 0..3 {
//            println!("{}...", i + 1);
//            std::thread::sleep(time::Duration::from_secs(1));
//        }
//
//        let id = rx.recv().await.expect("failed to get random id");
//
//        let id_str = utils::hash_to_string(&id);
//        println!("got random id: {id_str}");
//
//        // hash one more time to check how it behaves when listener receives not included id
//        let id = utils::sha256(Uuid::new_v4().to_string().as_bytes());
//
//        let mut stream = TcpStream::connect("127.0.0.1:8001")
//            .await
//            .expect("failed to connect");
//
//        // ping
//        println!("sending ping msg...");
//
//        NetOps::write(&mut stream, NetworkMessage::Ping)
//            .await
//            .expect("failed to send ping message");
//
//        NetOps::read(&mut stream)
//            .await
//            .expect("failed to receive pong...");
//
//        println!("successfully received pong message!");
//
//        // find node
//        NetOps::write(
//            &mut stream,
//            NetworkMessage::FindNode {
//                target_id: id.to_vec(),
//            },
//        )
//        .await
//        .expect("failed to write find node message");
//
//        if let Ok(NetworkMessage::FoundNode { nodes }) = NetOps::read(&mut stream).await {
//            for node in nodes {
//                println!("{node}");
//            }
//        } else {
//            println!("failed to receive found node message");
//        }
//    });
//
//    join_set.join_all().await;
//
//    Ok(())
//}
