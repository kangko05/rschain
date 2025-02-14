use std::error::Error;

use self::network::Node;

mod block;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let node = Node::new();
    dbg!(&node);

    node.listen().await;

    Ok(())
}
