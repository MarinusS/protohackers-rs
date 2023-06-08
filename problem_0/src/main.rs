use std::net::Ipv4Addr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    loop {
        let (mut socket, _addr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut buf = Vec::new();

            reader.read_to_end(&mut buf).await.unwrap();
            writer.write_all(buf.as_slice()).await.unwrap();
        });
    }
}
