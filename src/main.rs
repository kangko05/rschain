mod blockchain;
mod network;
mod utils;
mod wallet;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
