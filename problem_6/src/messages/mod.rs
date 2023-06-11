mod error;
mod i_am_camera;
mod plate;
mod ticket;
mod want_heartbeat;

mod factory;

pub use factory::*;

#[derive(PartialEq, Debug)]
pub enum Message {
    Error {
        msg: String,
    },
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
