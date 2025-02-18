#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use super::errors::NetResult;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use colored::Colorize;

/// handles basic network operations - reading stream, writing stream, ...
pub struct NetworkManager;

impl NetworkManager {
    pub async fn listen(port: u16) -> NetResult<()> {
        let socket_addr = Self::get_socket_addr(port)?;
        let listener = TcpListener::bind(socket_addr).await?;

        let mut handles = vec![];

        while let Ok((stream, addr)) = listener.accept().await {
            let handle = tokio::spawn(async move {
                Self::read_stream(stream, addr).await;
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Err(err) = handle.await {
                eprintln!("join error: {err}");
            };
        }

        Ok(())
    }

    // using locahost for this project
    fn get_socket_addr(port: u16) -> NetResult<SocketAddr> {
        let localhost = IpAddr::V4(Ipv4Addr::LOCALHOST);
        Ok(format!("{localhost}:{port}").parse()?)
    }

    async fn read_stream(mut stream: TcpStream, addr: SocketAddr) {
        loop {
            let mut buf = [0u8; 1024];
            if let Ok(n) = stream.read(&mut buf).await {
                match n {
                    0 => {
                        println!("{} disconnected", addr);
                        break;
                    }
                    _ => {
                        print!("from {}:", addr);
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        print!("got msg: {}", msg.green());
                        //dbg!(&buf[..n]);
                    }
                }
            };
        }
    }

    pub async fn send_message(port: u16) -> NetResult<()> {
        let socket_addr = Self::get_socket_addr(port)?;

        let mut stream = TcpStream::connect(socket_addr).await?;

        let stdin = std::io::stdin();

        loop {
            let mut buf = String::new();

            if let Err(err) = stdin.read_line(&mut buf) {
                eprintln!("failed to read line: {err}");
                break;
            };

            if buf.as_str().trim() == "exit\n" {
                break;
            }

            stream.write_all(buf.as_bytes()).await?;
        }

        Ok(())
    }
}
