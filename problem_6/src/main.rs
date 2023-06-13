use std::net::Ipv4Addr;

use problem_6::clients;
use problem_6::manager;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    let mut manager = manager::Manager::new();
    let comm_channels = manager.get_channels();

    tokio::spawn(async move { manager.run().await });

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        clients::new(socket, addr, comm_channels.clone());
    }
}
