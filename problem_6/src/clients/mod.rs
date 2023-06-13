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
use crate::messages::ServerMessage::Error as ErrorMessage;
use crate::messages::ServerMessage::Heartbeat;
use crate::{
    manager::{PlateObsv, PublicChannels},
    messages::ServerMessage::Ticket,
};

enum Error {
    ProtocolError { msg: String },
    ManagerCommFailed,
}

enum ClientType {
    Camera { road: u16, mile: u16, limit: u16 },
    Dispatcher { roads: Vec<u16> },
}

struct Client<'a> {
    addr: SocketAddr,
    manager_channels: PublicChannels,
    writer: WriteHalf<'a>,
    reader: BufReader<ReadHalf<'a>>,
    client_msg_fact: ClientMessageFactory,
    received_want_heart_beat: bool,
    client_type: Option<ClientType>,
    defribillator_tx: mpsc::Sender<u32>,
    heartbeat: mpsc::Receiver<()>,
    recv_ticket_chann_rx: mpsc::Receiver<Ticket>,
    recv_ticket_chann_tx: mpsc::Sender<Ticket>,
}

impl<'a> Client<'a> {
    fn new(
        socket: &'a mut TcpStream,
        addr: SocketAddr,
        manager_channels: PublicChannels,
    ) -> Client<'a> {
        let (reader, writer) = socket.split();
        let reader = BufReader::new(reader);

        let (defribillator_tx, defribillator_rx) = mpsc::channel(1);
        let heartbeat = new_heart(defribillator_rx);

        let client_msg_fact = ClientMessageFactory::new();

        let (recv_ticket_chann_tx, recv_ticket_chann_rx) = mpsc::channel(128);

        Client {
            addr,
            manager_channels,
            reader,
            writer,
            client_msg_fact,
            defribillator_tx,
            heartbeat,
            received_want_heart_beat: false,
            client_type: None,
            recv_ticket_chann_rx,
            recv_ticket_chann_tx,
        }
    }

    async fn msgs_handler(&mut self, msgs: Vec<ClientMessage>) -> Result<(), Error> {
        if msgs.is_empty() {
            return Ok(());
        }
        println!("Received msgs: {:?}", msgs);
        for msg in msgs {
            match msg {
                WantHeartbeat { interval } => {
                    if self.received_want_heart_beat {
                        return Err(Error::ProtocolError {
                            msg: "Already received WantHeartbeat".to_string(),
                        });
                    } else {
                        self.defribillator_tx.send(interval).await.unwrap();
                        self.received_want_heart_beat = true;
                    }
                }
                IAmCamera { road, mile, limit } => {
                    if self.client_type.is_some() {
                        return Err(Error::ProtocolError {
                            msg: "Already identified".to_string(),
                        });
                    } else {
                        self.client_type = Some(ClientType::Camera { road, mile, limit })
                    }
                }
                IAmDispatcher { roads } => {
                    if self.client_type.is_some() {
                        return Err(Error::ProtocolError {
                            msg: "Already identified".to_string(),
                        });
                    } else {
                        if self
                            .manager_channels
                            .dispatcher_regis_chan_tx
                            .send(crate::manager::DispatcherRegistration::Registration {
                                addr: self.addr,
                                roads: roads.clone(),
                                channel: self.recv_ticket_chann_tx.clone(),
                            })
                            .await
                            .is_err()
                        {
                            return Err(Error::ManagerCommFailed);
                        }
                        self.client_type = Some(ClientType::Dispatcher { roads })
                    }
                }
                Plate { plate, timestamp } => match self.client_type {
                    Some(ClientType::Camera { road, mile, limit }) => {
                        let plate_obsv = PlateObsv {
                            plate,
                            timestamp,
                            road,
                            mile,
                            speed_limit: limit,
                        };
                        if self
                            .manager_channels
                            .plate_obsv_chan_tx
                            .send(plate_obsv)
                            .await
                            .is_err()
                        {
                            return Err(Error::ManagerCommFailed);
                        }
                    }
                    _ => {
                        return Err(Error::ProtocolError {
                            msg: "Received Plate Message but client is not camera".to_string(),
                        })
                    }
                },
            }
        }

        Ok(())
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
                        Ok(msgs) => match self.msgs_handler(msgs).await {
                            Err(Error::ProtocolError { msg }) => {
                                println!("Error: Protocol error from client {:?}: {}", self.addr, msg);
                                self.writer.write_all(&ErrorMessage{msg}.encode()).await.unwrap();
                                break;
                            }
                            Err(Error::ManagerCommFailed) => {
                                println!("Error: Communication with master failed for client {:?}", self.addr);
                                break;
                            }

                            Ok(_) => {},
                        },
                        Err(err) => println!("Error while parsing message: {:?}", err),
                    };
                }

                Some(_) = self.heartbeat.recv() => {
                    self.writer.write_all(&Heartbeat::encode()).await.unwrap();
                }
                Some(ticket) = self.recv_ticket_chann_rx.recv() => {
                    println!("Sending ticket to netorwk");
                    self.writer.write_all(&ticket.encode()).await.unwrap();
                }
            }
        }

        if let Some(ClientType::Dispatcher { roads }) = &self.client_type {
            if self
                .manager_channels
                .dispatcher_regis_chan_tx
                .send(crate::manager::DispatcherRegistration::Deregister {
                    addr: self.addr,
                    roads: roads.clone(),
                })
                .await
                .is_err()
            {
                println!("Failed to communicate with manager to annouce disconnect")
            }
        }
    }
}

fn new_heart(mut defribillator_rx: mpsc::Receiver<u32>) -> mpsc::Receiver<()> {
    let (heart, stethoscope) = mpsc::channel(1);
    tokio::spawn(async move {
        if let Some(interval) = defribillator_rx.recv().await {
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
        }
    });

    stethoscope
}

pub fn new(mut socket: TcpStream, addr: SocketAddr, manager_channels: PublicChannels) {
    println!("New incomming connection: {:?}", addr);
    tokio::spawn(async move {
        let mut client = Client::new(&mut socket, addr, manager_channels);
        client.run().await;
    });
    println!("Closing connection with {:?}", addr);
}
