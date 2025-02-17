use std::error::Error;
use tokio::net::TcpStream;

use self::network::{BootstrapNode, NetMessage, NetOps};

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
        "server" => server().await,
        "client" => client().await,
        _ => panic!("burning"),
    }

    Ok(())
}

async fn server() {
    if let Err(err) = BootstrapNode::new().run().await {
        eprintln!("{err}");
    }
}

async fn client() {
    let mut stream = TcpStream::connect("127.0.0.1:8000").await.unwrap();
    //if let Err(err) = NetOps::write_message(&mut stream, NetMessage::GetChain).await {
    //    eprintln!("failed to write msg: {err}");
    //};

    if let Err(err) = NetOps::write_message(&mut stream, NetMessage::GetPeers).await {
        eprintln!("failed to write msg: {err}");
    };

    println!("sent")
}
