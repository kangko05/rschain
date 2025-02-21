use std::error::Error;

use self::network::Node;

mod blockchain;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut node1 = Node::new(8001);
    //let mut node2 = Node::new(8002);
    //let mut node3 = Node::new(8003);
    //
    //let node1_info = node1.get_node_info();
    //let node2_info = node2.get_node_info();
    //let node3_info = node3.get_node_info();

    println!("{node1}");

    Ok(())
}
