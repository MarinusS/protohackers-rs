mod factory;
mod i_am_camera;

pub use factory::*;

#[derive(PartialEq, Debug)]
pub enum Message {
    IAmCamera { road: u16, mile: u16, limit: u16 },
}
