use std::io::Error;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{lookup_host, TcpListener, TcpStream};

struct Message<'a> {
    result: Result<usize, Error>,
    msg: String,
    sender_addr: &'a SocketAddr,
    dest_addr: &'a SocketAddr,
    dest_writer: &'a mut OwnedWriteHalf,
}

async fn reroute_message<'a>(msg: &mut Message<'a>) -> bool {
    if msg.result.is_err() || *msg.result.as_ref().unwrap() == 0usize {
        println!(
            "{:?} broke the connection. Should break connection with {:?}",
            msg.sender_addr, msg.dest_addr
        );

        return true;
    }
    println!("Received from {:?}: {:?}", msg.sender_addr, msg.msg);

    let new_msg = msg.msg.clone();

    println!("Sending to {:?}: {:?}", msg.dest_addr, new_msg);
    msg.dest_writer
        .write_all(new_msg.as_bytes())
        .await
        .expect("Failed to send message");

    false
}

#[tokio::main]
async fn main() {
    let upstream_bind = ("chat.protohackers.com", 16963);
    let mut availabe_addr = lookup_host(upstream_bind)
        .await
        .expect("DNS request for upstream server failed");
    let up_addr = availabe_addr.next().expect("No DNS resolution");

    let local_bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(local_bind).await.unwrap();

    loop {
        let (down_socket, down_addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let (down_reader, mut down_writer) = down_socket.into_split();
            let upstream = TcpStream::connect(up_addr)
                .await
                .expect("Failed to conenct to upstream server");
            let (up_reader, mut up_writer) = upstream.into_split();

            let mut down_reader = BufReader::new(down_reader);
            let mut down_line_buf = String::new();
            let mut up_reader = BufReader::new(up_reader);
            let mut up_line_buf = String::new();

            loop {
                down_line_buf.clear();
                up_line_buf.clear();

                tokio::select! {
                result = down_reader.read_line(&mut down_line_buf) => {
                    let mut msg = Message{
                        result,
                        msg: down_line_buf.clone(),
                        sender_addr: &down_addr,
                        dest_addr: &up_addr,
                        dest_writer: &mut up_writer,
                    };

                    if reroute_message(&mut msg).await {
                        break;
                    }
                }


                result = up_reader.read_line(&mut up_line_buf) => {
                    let mut msg = Message{
                        result,
                        msg: up_line_buf.clone(),
                        sender_addr: &up_addr,
                        dest_addr: &down_addr,
                        dest_writer: &mut down_writer,
                    };

                    if reroute_message(&mut msg).await {
                        break;
                    }
                }
                }
            }
        });
    }
}
