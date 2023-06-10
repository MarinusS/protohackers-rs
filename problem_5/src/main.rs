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

static TONY_BOGUSCOIN_ADDR: &str = "7YWHMfk9JZe0LM0g1ZauHuiSxhI";

fn addr_of(s: &str) -> usize {
    s.as_ptr() as usize
}

fn split_whitespace_indices(s: &str) -> impl Iterator<Item = (usize, &str)> {
    s.split_whitespace()
        .map(move |sub| (addr_of(sub) - addr_of(s), sub))
}

fn is_boguscoin_addr(word: &str) -> bool {
    word.len() >= 26
        && word.len() <= 35
        && word.starts_with('7')
        && word.chars().all(|c| c.is_alphanumeric())
}

fn rewrite_boguscoin_addr(line: &str) -> String {
    //Iterator of tuples that are the starting and end posisionts of valid boguscoin adresses.
    let adresses_positions = split_whitespace_indices(line)
        .filter(|(_, word)| is_boguscoin_addr(word))
        .map(|(idx, word)| (idx, idx + word.len()));

    let (cursor, mut new_line) = adresses_positions.fold(
        (0, Vec::new()),
        |(cursor, mut new_line), (addr_start, addr_end)| {
            new_line.push(&line[cursor..addr_start]);
            new_line.push(TONY_BOGUSCOIN_ADDR);
            (addr_end, new_line)
        },
    );

    new_line.push(&line[cursor..]);
    new_line.concat()
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

    let new_msg = rewrite_boguscoin_addr(&msg.msg);

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
            let mut down_line_buf = Vec::new();
            let mut up_reader = BufReader::new(up_reader);
            let mut up_line_buf = Vec::new();

            loop {
                tokio::select! {
                    result = down_reader.read_until(b'\n', &mut down_line_buf) => {
                        let mut msg = Message{
                            result,
                            msg: String::from_utf8_lossy(&down_line_buf).to_string(),
                            sender_addr: &down_addr,
                            dest_addr: &up_addr,
                            dest_writer: &mut up_writer,
                        };
                        down_line_buf.clear();

                        if reroute_message(&mut msg).await {
                            break;
                        }
                    }


                    result = up_reader.read_until(b'\n', &mut up_line_buf) => {
                        let mut msg = Message{
                            result,
                            msg: String::from_utf8_lossy(&up_line_buf).to_string(),
                            sender_addr: &up_addr,
                            dest_addr: &down_addr,
                            dest_writer: &mut down_writer,
                        };
                        up_line_buf.clear();

                        if reroute_message(&mut msg).await {
                            break;
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_is_boguscoin_addr() {
        assert!(is_boguscoin_addr("7F1u3wSD5RbOHQmupo9nx4TnhQ"));
        assert!(is_boguscoin_addr("7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX"));
        assert!(is_boguscoin_addr("7LOrwbDlS8NujgjddyogWgIM93MV5N2VR"));
        assert!(is_boguscoin_addr("7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T"));
        assert!(!is_boguscoin_addr("7adNeSwJk@akpEcln9HEtthSRtxdmEHOT8T"));
        assert!(!is_boguscoin_addr("adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T"));
        assert!(!is_boguscoin_addr("adNeSwJkMakpEcln9HtxdmEHOT8T"));
        assert!(!is_boguscoin_addr("7adNeSwEHOT8T"));
        assert!(!is_boguscoin_addr(
            "7adNeSwasdkfjhkJLHHLKASJHDsdnfakjhaasdkfjhbaksdfEHOT8T"
        ));
    }

    #[test]
    fn test_rewrite_boguscoin_addr() {
        assert_eq!(
            rewrite_boguscoin_addr("7LOrwbDlS8NujgjddyogWgIM93MV5N2VR is my wallet address\n"),
            format!("{} is my wallet address\n", TONY_BOGUSCOIN_ADDR)
        );
        assert_eq!(
            rewrite_boguscoin_addr(
                "Please send the money to 7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T thanks\n"
            ),
            format!("Please send the money to {} thanks\n", TONY_BOGUSCOIN_ADDR)
        );
        assert_eq!(
            rewrite_boguscoin_addr(
                "Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX\n"
            ),
            format!("Hi alice, please send payment to {}\n", TONY_BOGUSCOIN_ADDR)
        );
    }
}
