use std::net::Ipv4Addr;
use tokio::net::UdpSocket;

use problem_7::*;

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let mut listener = LRCPListener::new(bind.0, bind.1)
        .await
        .expect("Failed to open LRCPlistener");
    let _sock = listener.accept().await;
}
