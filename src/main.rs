use std::error::Error;

mod blockchain;
mod network;
mod utils;
mod wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap()
        .to_string();

    Ok(())
}

//async fn run_bootstrap() -> NetworkResult<()> {
//    let mut bootstrap_node = BootstrapNode::new();
//    bootstrap_node.run().await
//}

//async fn run_full() -> NetworkResult<()> {
//    let mut full_node = FullNode::new(8003, "127.0.0.1:8333");
//    full_node.run().await
//}
//
//async fn run_client() {
//    let mut stream = TcpStream::connect("127.0.0.1:8080")
//        .await
//        .expect("client burning");
//
//    let mut buf = String::new();
//    let stdin = tokio::io::stdin();
//    let mut r = tokio::io::BufReader::new(stdin);
//
//    if let Err(err) = r.read_line(&mut buf).await {
//        eprintln!("{err}");
//    };
//
//    buf = buf.trim().to_string();
//
//    if !buf.is_empty() {
//        match buf.as_str() {
//            "getaddr" | "getaddr\n" => {
//                let msg = json!(NetworkMessage::GetAddr);
//                let msg = serde_json::to_vec(&msg).unwrap();
//                let length = (msg.len() as u32).to_be_bytes();
//
//                stream.write_all(&length).await.unwrap();
//                stream.write_all(&msg).await.unwrap();
//            }
//
//            "ping" | "ping\n" => {
//                let msg = json!(NetworkMessage::Ping);
//                let msg = serde_json::to_vec(&msg).unwrap();
//                let length = (msg.len() as u32).to_be_bytes();
//
//                stream.write_all(&length).await.unwrap();
//                stream.write_all(&msg).await.unwrap();
//            }
//            _ => {
//                if let Err(err) = stream.write_all(buf.as_bytes()).await {
//                    eprintln!("{err}");
//                } else {
//                    buf.clear();
//                }
//            }
//        }
//    }
//}
//
//async fn run_server() {
//    println!("running server");
//
//    let mut node = Node::new(8080);
//    node.test_run().await.expect("burn the server");
//}
