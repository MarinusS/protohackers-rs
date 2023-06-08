use std::net::Ipv4Addr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

struct TimestampedPrice {
    time_stamp: i32,
    price: i32,
}

fn average(numbers: &[i32]) -> i32 {
    if numbers.len() == 0 {
        return 0;
    }
    //Casting to i64 to prevent overflow
    let sum: i64 = numbers.iter().map(|x| *x as i64).sum();
    let count = numbers.len() as i64;
    (sum / count).try_into().unwrap()
}

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let mut inputs = Vec::new();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut buf = [0u8; 9];

            loop {
                if reader.read_exact(&mut buf).await.is_err() {
                    println! {"Received EOF from addr {}, clossing conncetion.", &addr}
                    break;
                }

                println!("Received from addr {}: {:?}", &addr, &buf);
                match buf[0] {
                    b'I' => inputs.push(TimestampedPrice {
                        time_stamp: i32::from_be_bytes(buf[1..=4].try_into().unwrap()),
                        price: i32::from_be_bytes(buf[5..=8].try_into().unwrap()),
                    }),
                    b'Q' => {
                        let min_time = i32::from_be_bytes(buf[1..=4].try_into().unwrap());
                        let max_time = i32::from_be_bytes(buf[5..=8].try_into().unwrap());
                        let average = average(
                            &inputs
                                .iter()
                                .filter(|ts_price| {
                                    min_time <= ts_price.time_stamp
                                        && ts_price.time_stamp <= max_time
                                })
                                .map(|ts_price| ts_price.price)
                                .collect::<Vec<i32>>(),
                        );
                        writer.write_all(&average.to_be_bytes()).await.unwrap();
                    }

                    _ => println!("Bad first byte"),
                }
            }
        });
    }
}
