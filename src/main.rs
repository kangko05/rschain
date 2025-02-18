mod blockchain;
mod network;
mod utils;
mod wallet;

use core::time;
use std::error::Error;
use std::thread::sleep;

use self::network::{BootstrapNode, NetMessage, NetOps};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap()
        .to_string();

    let arg1 = if let Some(s) = std::env::args().collect::<Vec<String>>().get(2) {
        s.to_string()
    } else {
        "".to_string()
    };

    match arg.as_str() {
        "cl" => run_client(&arg1).await,
        "li" => run_listener().await,

        _ => panic!("burn"),
    }

    Ok(())
}

async fn run_listener() {
    BootstrapNode::new().run().await
}

async fn run_client(msg: &str) {
    if let Err(err) = NetOps::send_message("127.0.0.1:8000", NetMessage::str(msg)).await {
        panic!("{err}")
    }

    sleep(time::Duration::from_secs(10));
}
