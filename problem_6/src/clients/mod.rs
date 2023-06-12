use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
    sync::mpsc,
};

use crate::messages::ClientMessage::{self, *};
use crate::messages::ClientMessageFactory;
use crate::messages::ServerMessage::Heartbeat;

enum ClientType {
    Camera { road: u16, mile: u16, limit: u16 },
    Dispatcher { roads: Vec<u16> },
}

struct Client<'a> {
    addr: SocketAddr,
    writer: WriteHalf<'a>,
    reader: BufReader<ReadHalf<'a>>,
    client_msg_fact: ClientMessageFactory,
    received_want_heart_beat: bool,
    client_type: Option<ClientType>,
    defribillator_tx: mpsc::Sender<u32>,
    heartbeat: mpsc::Receiver<()>,
}

impl<'a> Client<'a> {
    fn new(socket: &'a mut TcpStream, addr: SocketAddr) -> Client<'a> {
        let (reader, writer) = socket.split();
        let reader = BufReader::new(reader);

        let (defribillator_tx, defribillator_rx) = mpsc::channel(1);
        let heartbeat = new_heart(defribillator_rx);

        let client_msg_fact = ClientMessageFactory::new();

        Client {
            addr,
            reader,
            writer,
            client_msg_fact,
            defribillator_tx,
            heartbeat,
            received_want_heart_beat: false,
            client_type: None,
        }
    }

    async fn msgs_handler(&mut self, msgs: Vec<ClientMessage>) {
        if !msgs.is_empty() {
            println!("Received msgs: {:?}", msgs);
            for msg in msgs {
                match msg {
                    WantHeartbeat { interval } => {
                        self.defribillator_tx.send(interval).await.unwrap();
                    }
                    IAmCamera { road, mile, limit } => {
                        self.client_type = Some(ClientType::Camera { road, mile, limit })
                    }
                    IAmDispatcher { roads } => {
                        self.client_type = Some(ClientType::Dispatcher { roads })
                    }
                    Plate { plate, timestamp } => todo!(),
                }
            }
        }
    }

    async fn run(&'a mut self) {
        let mut buf = Vec::new();

        loop {
            tokio::select! {
                bytes_read = self.reader.read_buf(&mut buf) => {
                    if bytes_read.is_err() || bytes_read.unwrap() == 0{
                        break;
                    }
                    let msgs = self.client_msg_fact.push(&buf);
                    buf.clear();
                    match msgs {
                        Ok(msgs) => self.msgs_handler(msgs).await,
                        Err(err) => println!("Err: {:?}", err),
                    };
                }

                Some(_) = self.heartbeat.recv() => {
                    self.writer.write_all(&Heartbeat::encode()).await.unwrap();
                }
            }
        }
    }
}

fn new_heart(mut defribillator_rx: mpsc::Receiver<u32>) -> mpsc::Receiver<()> {
    let (heart, stethoscope) = mpsc::channel(1);
    tokio::spawn(async move {
        let interval = defribillator_rx.recv().await.unwrap();
        if interval > 0 {
            let seconds = interval as u64 / 10;
            let nanos = (interval % 10) * 1_000_000_000;

            let interval = tokio::time::Duration::new(seconds, nanos);
            loop {
                tokio::time::sleep(interval).await;
                if heart.send(()).await.is_err() {
                    break;
                }
            }
        }
    });

    stethoscope
}

pub fn new(mut socket: TcpStream, addr: SocketAddr) {
    println!("New incomming connection: {:?}", addr);
    tokio::spawn(async move {
        let mut client = Client::new(&mut socket, addr);
        client.run().await;
    });
    println!("Closing connection with {:?}", addr);
}
