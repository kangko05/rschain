use core::time;
use std::error::Error;
use std::thread::sleep;
use tokio::net::TcpStream;

use self::network::{BootstrapNode, FullNode, NetMessage, NetOps, RunNode};

mod blockchain;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap()
        .to_string();

    match arg.as_str() {
        "full" => full().await,
        "server" => bootstrap().await,
        "client" => client().await,
        _ => panic!("burning"),
    }

    Ok(())
}

async fn bootstrap() {
    if let Err(err) = BootstrapNode::new().run().await {
        eprintln!("{err}");
    }
}

async fn full() {
    let bootstrap_handle = tokio::spawn(async move {
        println!("running bootstrap node");
        bootstrap().await;
    });

    let fullnode_handle = tokio::spawn(async move {
        for i in 0..3 {
            println!("{}...", i + 1);
            sleep(time::Duration::from_secs(1));
        }

        println!("running full node");

        let _ = FullNode::init(8001)
            .await
            .expect("failed to init full node");
    });

    let _ = tokio::join!(bootstrap_handle, fullnode_handle);
}

async fn client() {
    let mut stream = TcpStream::connect("127.0.0.1:8000").await.unwrap();
    //if let Err(err) = NetOps::write_message(&mut stream, NetMessage::GetChain).await {
    //    eprintln!("failed to write msg: {err}");
    //};

    if let Err(err) = NetOps::write_message(&mut stream, NetMessage::GetChain).await {
        eprintln!("failed to write msg: {err}");
    };

    let res = NetOps::read_stream(&mut stream).await.expect("1");

    let parsed = serde_json::from_slice::<NetMessage>(&res).expect("2");

    dbg!(parsed);
}
