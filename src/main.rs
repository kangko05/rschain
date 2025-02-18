mod blockchain;
mod network;
mod utils;
mod wallet;

use std::error::Error;

use self::network::NetworkManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap()
        .to_string();

    match arg.as_str() {
        "cl" => run_client().await,
        "li" => run_listener().await,

        _ => panic!("burn"),
    }

    Ok(())
}

async fn run_listener() {
    if let Err(err) = NetworkManager::listen(8001).await {
        panic!("{}", err);
    };
}

async fn run_client() {
    if let Err(err) = NetworkManager::send_message(8001).await {
        panic!("{err}")
    }
}
