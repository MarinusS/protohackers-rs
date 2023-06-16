use std::{collections::HashMap, mem::size_of, net::SocketAddr, time::Duration};

use tokio::{io, net::UdpSocket, sync::mpsc, task::JoinHandle};

type SessionToken = u32;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Connect {
        token: SessionToken,
    },
    Data {
        token: SessionToken,
        pos: u64,
        data: String,
    },
    Ack {
        token: SessionToken,
        length: u64,
    },
    Close {
        token: SessionToken,
    },
}

impl Message {
    fn encode(self) -> Vec<u8> {
        match self {
            Message::Connect { token } => {
                unreachable!("server will never need to initiate the opening of any sessions")
            }
            Message::Data { token, pos, data } => {
                format!("/data/{token}/{pos}/{data}/").into_bytes()
            }
            Message::Ack { token, length } => format!("/ack/{token}/{length}/").into_bytes(),
            Message::Close { token } => format!("/close/{token}/").into_bytes(),
        }
    }
}

#[derive(Debug, Clone)]
struct LCRPSession {
    token: SessionToken,
    addr: SocketAddr,
    pos_counter: u64,
    udp_out_chan_tx: mpsc::Sender<(Message, SocketAddr)>,
}

impl LCRPSession {
    async fn new(
        token: SessionToken,
        addr: SocketAddr,
        udp_out_chan_tx: mpsc::Sender<(Message, SocketAddr)>,
    ) -> Self {
        println!("New LCRPSession from addr {addr} with token: {token}");
        udp_out_chan_tx
            .send((Message::Ack { token, length: 0 }, addr))
            .await
            .unwrap();

        LCRPSession {
            token,
            addr,
            pos_counter: 0,
            udp_out_chan_tx,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LCRPsocket {
    pub token: SessionToken,
    pub addr: SocketAddr,
}

pub struct LRCPListener {
    new_session_chan_rx: mpsc::Receiver<LCRPsocket>,
    udp_manager_task: JoinHandle<()>,
    lcrp_manager_task: JoinHandle<()>,
}

impl LRCPListener {
    pub async fn new(ip_addr: std::net::Ipv4Addr, port: u16) -> Result<LRCPListener, io::Error> {
        let udp_socket = UdpSocket::bind((ip_addr, port)).await?;
        println!("Opened udp socket on addr: {:?}", udp_socket.local_addr());
        let (new_session_chan_tx, new_session_chan_rx) = mpsc::channel(64);
        let (new_message_chan_tx, new_message_chan_rx) = mpsc::channel(64);
        let (udp_out_chan_tx, udp_out_chan_rx) = mpsc::channel(64);

        let udp_listener_task = tokio::spawn(udp_listener(
            udp_socket,
            new_message_chan_tx,
            udp_out_chan_rx,
        ));
        let lcrp_manager_task = tokio::spawn(lcrp_manager(
            new_message_chan_rx,
            new_session_chan_tx,
            udp_out_chan_tx,
        ));

        Ok(LRCPListener {
            new_session_chan_rx,
            udp_manager_task: udp_listener_task,
            lcrp_manager_task,
        })
    }

    pub async fn accept(&mut self) -> LCRPsocket {
        self.new_session_chan_rx.recv().await.unwrap()
    }
}

impl Drop for LRCPListener {
    fn drop(&mut self) {
        self.udp_manager_task.abort();
        self.lcrp_manager_task.abort();
    }
}

async fn lcrp_manager(
    mut new_message_chan_rx: mpsc::Receiver<(Message, SocketAddr)>,
    new_session_chan_tx: mpsc::Sender<LCRPsocket>,
    udp_out_chan_tx: mpsc::Sender<(Message, SocketAddr)>,
) {
    let mut sessions = HashMap::new();

    loop {
        match new_message_chan_rx.recv().await.unwrap() {
            (Message::Connect { token }, addr) => {
                let new_session = LCRPSession::new(token, addr, udp_out_chan_tx.clone()).await;
                sessions.insert(token, new_session);

                let sock = LCRPsocket { token, addr };
                new_session_chan_tx.send(sock).await.unwrap();
            }
            (Message::Data { token, pos, data }, _) => {
                if !sessions.contains_key(&token) {
                    todo!("Received data for a not open sessions. Send close");
                }
                todo!("Implement receive data");
            }
            (Message::Ack { token, length }, _) => {
                if !sessions.contains_key(&token) {
                    todo!("Received ack for a not open sessions. Send close");
                }
                todo!("Implement receive ack");
            }
            (Message::Close { token }, _) => {
                if !sessions.contains_key(&token) {
                    todo!("Received close for a not open sessions. Send close");
                }
                todo!("Implement receive close");
            }
        }
    }
}

async fn udp_listener(
    socket: UdpSocket,
    new_message_chan_tx: mpsc::Sender<(Message, SocketAddr)>,
    mut udp_out_chan_rx: mpsc::Receiver<(Message, SocketAddr)>,
) {
    let mut buf = [0; 1024];
    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                let (len, addr) = result.unwrap();

                let msg = parse_datagram(&buf[..len]);
                if let Some(msg) = msg {
                    new_message_chan_tx.send((msg, addr)).await.unwrap();
                }
            }

            result = udp_out_chan_rx.recv() => {
                let (msg, addr) = result.unwrap();
                let encoded = msg.encode();
                let bytes_to_send = encoded.len();
                let len = socket.send_to(&encoded, addr).await.unwrap();

                if bytes_to_send != len {
                    panic!("Not all bytes where sent to udp socket and this is not handled yet");
                }
            }
        }
    }
}

fn parse_numfield_as_acii<T: std::str::FromStr>(num: &[u8]) -> Option<T> {
    str::parse::<T>(std::str::from_utf8(num).ok()?).ok()
}

fn parse_datagram(datagram: &[u8]) -> Option<Message> {
    if !(datagram.first() == Some(&b'/') && datagram.last() == Some(&b'/')) {
        return None;
    }

    let datagram = &datagram[1..datagram.len() - 1];
    let mut values = datagram.split(|&x| x == b'/');

    match values.next() {
        Some(b"connect") => Some(Message::Connect {
            token: parse_numfield_as_acii(values.next()?)?,
        }),
        Some(b"data") => Some(Message::Data {
            token: parse_numfield_as_acii(values.next()?)?,
            pos: parse_numfield_as_acii(values.next()?)?,
            data: std::str::from_utf8(values.next()?).ok()?.to_string(),
        }),
        Some(b"ack") => Some(Message::Ack {
            token: parse_numfield_as_acii(values.next()?)?,
            length: parse_numfield_as_acii(values.next()?)?,
        }),
        Some(b"close") => Some(Message::Close {
            token: parse_numfield_as_acii(values.next()?)?,
        }),
        _ => {
            eprintln!("Received unknown message");
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use tokio::task::JoinSet;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn test_connect_ack_single_client() {
        let server_addr = (Ipv4Addr::LOCALHOST, 8080);
        let client_addr = (Ipv4Addr::LOCALHOST, 10000);

        let session_token = 12345;

        let server_handle = tokio::spawn(async move {
            let mut listener = LRCPListener::new(server_addr.0, server_addr.1)
                .await
                .expect("Failed to open LRCPlistener");
            let _sock = listener.accept().await;
        });

        let client_handle = tokio::spawn(async move {
            let token = session_token;
            let sock = UdpSocket::bind(client_addr)
                .await
                .expect("Simulated client failed to bind to udp socket");

            sock.send_to(format!("/connect/{token}/").as_bytes(), server_addr)
                .await
                .unwrap();

            let mut buf = [0; 1024];
            let (len, _addr) = sock.recv_from(&mut buf).await.unwrap();
            parse_datagram(&buf[..len])
        });

        server_handle.await.unwrap();
        let recv_by_client = client_handle.await.unwrap();

        assert_eq!(
            Some(Message::Ack {
                token: session_token,
                length: 0
            }),
            recv_by_client
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_connect_ack_multiple_clients() {
        let server_addr = (Ipv4Addr::LOCALHOST, 8080);

        let num_clients = 15;
        let clients_addr_ip = Ipv4Addr::LOCALHOST;
        let clients_addr_port_strt = 10000;
        let sessions_token_strt = 12345;

        let server_handle = tokio::spawn(async move {
            let mut listener = LRCPListener::new(server_addr.0, server_addr.1)
                .await
                .expect("Failed to open LRCPlistener");
            loop {
                let sock = listener.accept().await;
                tokio::spawn(async move {
                    let _sock = sock;
                });
            }
        });

        let mut clients_set = JoinSet::new();
        for i in 0..num_clients {
            clients_set.spawn(async move {
                let token = sessions_token_strt + i as u32;
                let sock = UdpSocket::bind((clients_addr_ip, clients_addr_port_strt + i))
                    .await
                    .expect("Simulated client failed to bind to udp socket");

                sock.send_to(format!("/connect/{token}/").as_bytes(), server_addr)
                    .await
                    .unwrap();

                let mut buf = [0; 1024];
                let (len, _addr) = sock.recv_from(&mut buf).await.unwrap();
                (token, parse_datagram(&buf[..len]))
            });
        }

        while let Some(res) = clients_set.join_next().await {
            let (token, ack) = res.unwrap();

            assert_eq!(Some(Message::Ack { token, length: 0 }), ack);
        }

        assert!(!server_handle.is_finished());
        server_handle.abort();
    }
}
