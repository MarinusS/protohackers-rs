use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
#[derive(Serialize, Deserialize, Debug)]
struct IsPrimeRequest {
    method: String,
    number: Number,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsPrimeResponse {
    method: String,
    prime: bool,
}

fn is_prime(number: u64) -> bool {
    if number <= 1 {
        return false;
    }

    for i in 2..=(number as f64).sqrt().floor() as u64 {
        if number % i == 0 {
            return false;
        }
    }
    true
}

fn is_prime_handler(request: Value) -> Vec<u8> {
    let bad_request_response: Vec<u8> = vec![b'\n'];

    match serde_json::from_value::<IsPrimeRequest>(request) {
        Ok(request) => {
            let prime = if request.number.is_u64() {
                is_prime(request.number.as_u64().unwrap())
            } else {
                false
            };

            let mut response = serde_json::to_string(&IsPrimeResponse {
                method: "isPrime".to_string(),
                prime,
            })
            .unwrap();
            response.push('\n');
            response.into_bytes()
        }
        Err(_) => bad_request_response,
    }
}

fn request_handler(line: &str) -> Vec<u8> {
    let bad_request_response: Vec<u8> = vec![b'\n'];

    if let Ok(request) = serde_json::from_str::<Value>(line) {
        if request["method"].is_string() {
            match request["method"].as_str().unwrap() {
                "isPrime" => return is_prime_handler(request),
                str => println!("Unknown method requested: {}", str),
            }
        }
    };

    bad_request_response
}

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);

            let mut buf = String::new();
            loop {
                buf.clear();
                let bytes_read = reader.read_line(&mut buf).await.unwrap();
                println!("Received line from addr {}: {:?}", &addr, &buf);
                if bytes_read == 0 {
                    break;
                }

                let response = request_handler(&buf);
                println!(
                    "Sending to addr {}: {:?}",
                    &addr,
                    String::from_utf8_lossy(&response)
                );
                writer.write_all(&response).await.unwrap();
            }
        });
    }
}
