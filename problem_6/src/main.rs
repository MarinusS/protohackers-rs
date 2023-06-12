use std::net::Ipv4Addr;

use messages::ClientMessageFactory;
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::TcpListener,
};

mod messages;

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    loop {
        let (mut socket, _addr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            let (reader, mut _writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut buf = Vec::new();

            let mut fact = ClientMessageFactory::new();
            reader.read_to_end(&mut buf).await.unwrap();
            let msg = fact.push(&buf);

            match msg {
                Ok(msg) => {
                    if !msg.is_empty() {
                        println!("Received msgs: {:?}", msg)
                    }
                }
                Err(err) => println!("Err: {:?}", err),
            };

            buf.clear();
        });
    }
}
