//#![allow(dead_code, unused)]

use core::time;
use std::error::Error;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinSet;

use self::network::Node;

mod blockchain;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut join_set = JoinSet::new();

    // 첫 번째 노드 (시드 노드)
    let node1 = Node::new(8001);
    let node1_info = node1.get_node_info();
    println!("Node 1 ID: {}", utils::hash_to_string(node1_info.get_id()));
    let node1_arc = Arc::new(RwLock::new(node1));
    join_set.spawn(async move {
        Node::run(node1_arc).await;
    });

    // 잠시 기다려서 첫 번째 노드가 준비되도록 함
    std::thread::sleep(time::Duration::from_secs(2));

    // 추가 노드들 생성 (모두 첫 번째 노드를 시드로 사용)
    let node_count = 5; // 시드 노드 제외한 추가 노드 수
    let mut node_infos = vec![node1_info.clone()];

    for i in 0..node_count {
        let port = 8002 + i;

        // 노드 생성 및 부트스트랩
        let mut node = Node::new(port);
        let node_info = node.get_node_info();
        println!(
            "Node {} ID: {}",
            i + 2,
            utils::hash_to_string(node_info.get_id())
        );

        // 기존 모든 노드 정보로 부트스트랩
        if let Err(err) = node.bootstrap(&node_infos).await {
            eprintln!("Failed to bootstrap node {}: {}", i + 2, err);
        } else {
            println!("Node {} bootstrapped successfully", i + 2);
        }

        // 이 노드 정보를 목록에 추가 (다음 노드의 부트스트랩에 사용)
        node_infos.push(node_info);

        // 노드 실행
        let node_arc = Arc::new(RwLock::new(node));
        join_set.spawn(async move {
            Node::run(node_arc).await;
        });

        // 약간의 지연 추가 (부트스트랩 과정이 겹치지 않도록)
        std::thread::sleep(time::Duration::from_millis(500));
    }

    // 모든 노드가 시작된 후 검색 테스트
    std::thread::sleep(time::Duration::from_secs(3));
    println!("\n--- Testing node lookup ---");

    // 새 노드로 마지막 노드 검색 시도
    let mut test_node = Node::new(9000);
    let target_id = node_infos.last().unwrap().get_id().clone();
    println!(
        "Looking for node with ID: {}",
        utils::hash_to_string(&target_id)
    );

    // 첫 번째 노드만 알고 있는 상태에서 시작
    if let Err(err) = test_node.bootstrap(&[node_infos[0].clone()]).await {
        eprintln!("Failed to bootstrap test node: {}", err);
    }

    // 검색 시도
    if let Some(found) = test_node
        .node_lookup(&[node_infos[0].clone()], &target_id, 20, false)
        .await
    {
        println!("✅ Successfully found target node!");
        println!("Found node: {}", found);
    } else {
        println!("❌ Failed to find target node");
    }

    join_set.join_all().await;
    Ok(())
}

//#[tokio::main]
//async fn main() -> Result<(), Box<dyn Error>> {
//    let mut join_set = JoinSet::new();
//
//    // first node
//    let node1 = Node::new(8001);
//    let node1_info = node1.get_node_info();
//    let node1_arc = Arc::new(RwLock::new(node1));
//
//    join_set.spawn(async move {
//        Node::run(node1_arc).await;
//    });
//
//    // second node
//    std::thread::sleep(time::Duration::from_secs(3));
//    let mut node2 = Node::new(8002);
//    let node2_info = node2.get_node_info();
//    if let Err(err) = node2.bootstrap(&[node1_info.clone()]).await {
//        eprintln!("failed to bootstrap node2: {err}");
//    };
//
//    //let node2_info = node2.get_node_info();
//    let node2_arc = Arc::new(RwLock::new(node2));
//
//    join_set.spawn(async move {
//        Node::run(node2_arc).await;
//    });
//
//    // third node
//    std::thread::sleep(time::Duration::from_secs(3));
//    let mut node3 = Node::new(8003);
//    if let Err(err) = node3.bootstrap(&[node1_info, node2_info]).await {
//        eprintln!("failed to bootstrap node2: {err}");
//    };
//
//    //let node2_info = node2.get_node_info();
//    let node3_arc = Arc::new(RwLock::new(node3));
//
//    join_set.spawn(async move {
//        Node::run(node3_arc).await;
//    });
//
//    join_set.join_all().await;
//
//    Ok(())
//}

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
