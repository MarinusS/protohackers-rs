mod error;
mod heartbeat;
mod i_am_camera;
mod plate;
mod ticket;
mod want_heartbeat;

mod client_message_factory;

pub use client_message_factory::*;

//TODO Implement IAmDispatcher
#[derive(PartialEq, Debug)]
pub enum ClientMessage {
    Plate {
        plate: String,
        timestamp: u32,
    },
    Ticket {
        plate: String,
        road: u16,
        mile1: u16,
        timestamp1: u32,
        mile2: u16,
        timestamp2: u32,
        speed: u16,
    },
    WantHeartbeat {
        interval: u32,
    },
    IAmCamera {
        road: u16,
        mile: u16,
        limit: u16,
    },
}

//TODO Implement HeartBeat
#[derive(PartialEq, Debug)]
pub enum ServerMessage {
    Error { msg: error::Error },
    Heartbeat { msg: heartbeat::Hearbeat },
}
