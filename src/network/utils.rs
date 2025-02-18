use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub fn get_socket_addr(port: u16) -> SocketAddr {
    let localhost = IpAddr::V4(Ipv4Addr::LOCALHOST);

    format!("{}:{}", localhost, port)
        .parse()
        .expect("shouldn't fail parsing localhost string into socket addr")
}
