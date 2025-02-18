mod blockchain;
mod network;
mod utils;
mod wallet;

use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use self::network::{BootstrapNode, FullNode, RunNode};
use self::wallet::Wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let wallet1 = Wallet::new();
    let addr1 = wallet1.get_address();

    let args = std::env::args().collect::<Vec<String>>();
    let args = &args[1..];

    let arg = args[0].to_string();

    let port: u16 = if args.len() > 1 {
        args[1].parse::<u16>().unwrap()
    } else {
        0
    };

    match arg.as_str() {
        "boot" => boot().await,
        "full" => full(port, addr1).await,
        _ => panic!("burning"),
    }

    Ok(())
}

async fn boot() {
    println!("running botstrap node");
    BootstrapNode::new()
        .run()
        .await
        .expect("failed to run bootstrap node");
}

async fn full(port: u16, to_addr: &str) {
    let from_wallet = get_test_wallet().await;

    let fullnode = FullNode::init(port)
        .await
        .unwrap_or_else(|err| panic!("failed to init full node at port {port}: {err}"));

    let handles = fullnode
        .run()
        .unwrap_or_else(|err| panic!("failed to run full node at port {port}: {err}"));

    if port == 8002 {
        fullnode
            .send_tx(&from_wallet, to_addr, 10)
            .await
            .unwrap_or_else(|err| panic!("failed to send tx from {}: {}", port, err));

        println!("sent new tx");
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

async fn get_test_wallet() -> Wallet {
    let mut stream = TcpStream::connect("127.0.0.1:3000").await.unwrap();
    stream.write_all("addr".as_bytes()).await.unwrap();

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();

    serde_json::from_slice::<Wallet>(&buf[..n]).unwrap()
}
