use std::{collections::HashMap, net::Ipv4Addr};

use tokio::net::UdpSocket;

static VERSION: &str = "TOM THE GENIUS Key-Value Store 1.0";

#[derive(Debug)]
enum Request<'a> {
    Insert { key: &'a str, value: &'a str },
    Retrieve { key: &'a str },
}

fn build_request(data: &str) -> Request<'_> {
    match data
        .chars()
        .enumerate()
        .find(|(_, char)| char == &'=')
        .map(|(idx, _)| idx)
    {
        Some(idx) => Request::Insert {
            key: &data[..idx],
            value: &data[idx + 1..],
        },

        None => Request::Retrieve { key: data },
    }
}

//I am not spawning any threads and dealing with the UDP packets sequentially
//Therefore this could have easily been solved without tokio
//I might come back and send the responses in threads to make things more interesting
//
#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let socket = UdpSocket::bind(bind).await.unwrap();

    let my_addr = socket.local_addr().unwrap();
    println!("Listening on: {}", my_addr);

    let mut data = HashMap::new();
    data.insert("version".to_string(), VERSION.to_string());

    let mut read_buf = [0u8; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut read_buf).await.unwrap();
        println!(
            "{:?} bytes received from {:?}: {:?}",
            len,
            addr,
            &read_buf[..len]
        );

        let request = build_request(std::str::from_utf8(&read_buf[..len]).unwrap());
        println!("Request: {:?}", request);

        match request {
            Request::Insert { key, value } => {
                if key != "version" {
                    data.insert(key.to_string(), value.to_string());
                }
            }
            Request::Retrieve { key } => {
                let msg = format!("{}={}", key, data.get(key).unwrap_or(&"".to_string()));
                println!("Sending to {}: {}", addr, msg);
                socket.send_to(msg.as_bytes(), addr).await.unwrap();
            }
        }
    }
}
