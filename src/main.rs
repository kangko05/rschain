use std::error::Error;

use serde_json::json;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpStream,
};

use network::{NetworkMessage, Node};

mod block;
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
        "client" => run_client().await,
        "server" => run_server().await,
        _ => panic!("burn in fire"),
    }

    Ok(())
}

async fn run_client() {
    let mut stream = TcpStream::connect("127.0.0.1:8080")
        .await
        .expect("client burning");

    let mut buf = String::new();
    let stdin = tokio::io::stdin();
    let mut r = tokio::io::BufReader::new(stdin);

    while buf != "exit\n" {
        if let Err(err) = r.read_line(&mut buf).await {
            eprintln!("{err}");
            continue;
        };

        buf = buf.trim().to_string();

        if !buf.is_empty() {
            match buf.as_str() {
                "getaddr" | "getaddr\n" => {
                    let msg = json!(NetworkMessage::GetAddr);
                    let msg = serde_json::to_vec(&msg).unwrap();

                    stream.write_all(&msg).await.unwrap();
                }
                _ => {
                    if let Err(err) = stream.write_all(buf.as_bytes()).await {
                        eprintln!("{err}");
                    } else {
                        buf.clear();
                    }
                }
            }
        }
    }
}

async fn run_server() {
    println!("running server");

    let mut node = Node::new(8080);
    node.test_run().await.expect("burn the server");
}
