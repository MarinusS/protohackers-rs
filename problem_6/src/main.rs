use std::net::Ipv4Addr;

use problem_6::clients::new;
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        new(socket, addr);
    }
}
