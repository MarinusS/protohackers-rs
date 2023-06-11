mod error;
mod i_am_camera;
mod plate;

mod factory;

pub use factory::*;

#[derive(PartialEq, Debug)]
pub enum Message {
    Error { msg: String },
    Plate { plate: String, timestamp: u32 },
    IAmCamera { road: u16, mile: u16, limit: u16 },
}
