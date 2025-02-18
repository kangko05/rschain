mod blockchain;
mod network;
mod utils;
mod wallet;

use std::error::Error;

use network::Node;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = Node::init().await;

    Ok(())
}
