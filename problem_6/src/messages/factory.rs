use super::plate::PlateFactory;
use super::ticket::{self, TicketFactory};
use super::want_heartbeat::WantHeartbeatFactory;
use super::{plate, want_heartbeat, Message};
use std::collections::VecDeque;

use super::error;
use super::error::ErrorFactory;
use super::i_am_camera;
use super::i_am_camera::IAmCameraFactory;

#[derive(Debug)]
enum ParsingErrorType {
    UnknowMessageType { id: String },
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ParsingError {
    error_type: ParsingErrorType,
}

pub struct Factory {
    buffer: VecDeque<u8>,
    curr_factory: Option<Box<dyn MessageFactory + Send>>,
}

pub trait MessageFactory {
    fn new() -> Self
    where
        Self: Sized;
    fn push(&mut self, data: u8) -> Option<Message>;
}

impl Factory {
    pub fn new() -> Self {
        Factory {
            buffer: VecDeque::new(),
            curr_factory: None,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Result<Vec<Message>, ParsingError> {
        let mut new_messages = Vec::new();
        self.buffer.extend(data.iter());

        while !self.buffer.is_empty() {
            if self.curr_factory.is_none() {
                self.curr_factory = Some(new_sub_factory(self.buffer.pop_front().unwrap())?);
            } else {
                let curr_factory = &mut self.curr_factory;
                let buffer = &mut self.buffer;

                while !buffer.is_empty() && curr_factory.is_some() {
                    let new_message = curr_factory
                        .as_mut()
                        .unwrap()
                        .push(buffer.pop_front().unwrap());

                    if let Some(new_message) = new_message {
                        new_messages.push(new_message);
                        *curr_factory = None;
                    }
                }
            }
        }

        Ok(new_messages)
    }
}

fn new_sub_factory(id_byte: u8) -> Result<Box<dyn MessageFactory + Send>, ParsingError> {
    match id_byte {
        i_am_camera::ID_BYTE => Ok(Box::new(IAmCameraFactory::new())),
        error::ID_BYTE => Ok(Box::new(ErrorFactory::new())),
        plate::ID_BYTE => Ok(Box::new(PlateFactory::new())),
        ticket::ID_BYTE => Ok(Box::new(TicketFactory::new())),
        want_heartbeat::ID_BYTE => Ok(Box::new(WantHeartbeatFactory::new())),
        _ => Err(ParsingError {
            error_type: ParsingErrorType::UnknowMessageType {
                id: id_byte.to_string(),
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_push_one_byte() {
        use super::Message::*;
        struct Test {
            test_data: Vec<u8>,
            expected: Vec<Vec<Message>>,
        }

        let tests = vec![Test {
            test_data: vec![
                0x80, 0x00, 0x42, 0x00, 0x64, 0x00, 0x3c, 0x80, 0x01, 0x70, 0x04, 0xd2, 0x00, 0x28,
            ],
            expected: vec![
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![IAmCamera {
                    road: 66,
                    mile: 100,
                    limit: 60,
                }],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![IAmCamera {
                    road: 368,
                    mile: 1234,
                    limit: 40,
                }],
            ],
        }];

        for test in tests {
            let mut fact = Factory::new();

            for (i, &data) in test.test_data.iter().enumerate() {
                let messages = fact.push(&[data]);

                assert!(messages.is_ok());
                let messages = messages.unwrap();
                assert_eq!(messages, test.expected[i]);
            }
        }
    }

    #[test]
    fn test_push_in_slices() {
        use super::Message::*;
        struct Test<'a> {
            test_data: Vec<&'a [u8]>,
            expected: Vec<Vec<Message>>,
        }

        let tests = vec![Test {
            test_data: vec![
                &[0x80, 0x00, 0x42], //Starting IamCamera
                &[0x00, 0x64, 0x00],
                &[0x3c, 0x80, 0x01], //Finished IamCamera and starting new IAmCamera
                &[0x70, 0x04, 0xd2],
                &[0x00, 0x28, 0x10], //Finished IamCamera and starting new Error
                &[0x03, 0x62, 0x61, 0x64], //Finished Error
                &[
                    0x21, 0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x42, 0x00, 0x64, 0x00, 0x01, 0xe2,
                    0x40, 0x00, 0x6e, 0x00, 0x01, 0xe3, 0xa8, 0x27, 0x10,
                ], // Start and finish Ticket
                &[0x20, 0x04, 0x55, 0x4e, 0x31, 0x58], //Start Plate
                &[
                    0x00, 0x00, 0x03, 0xe8, 0x21, 0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47,
                    0x01, 0x70, 0x04, 0xd2, 0x00, 0x0f, 0x42, 0x40, 0x04, 0xd3, 0x00, 0x0f, 0x42,
                    0x7c, 0x17, 0x70, 0x40, 0x00, 0x00, 0x04, 0xdb,
                ], //Finish Plate, Start and Finish Ticket, Start and Finish Heartbeat
            ],
            expected: vec![
                Vec::new(),
                Vec::new(),
                vec![IAmCamera {
                    road: 66,
                    mile: 100,
                    limit: 60,
                }],
                Vec::new(),
                vec![IAmCamera {
                    road: 368,
                    mile: 1234,
                    limit: 40,
                }],
                vec![Error {
                    msg: "bad".to_string(),
                }],
                vec![Ticket {
                    plate: "UN1X".to_string(),
                    road: 66,
                    mile1: 100,
                    timestamp1: 123456,
                    mile2: 110,
                    timestamp2: 123816,
                    speed: 10000,
                }],
                Vec::new(),
                vec![
                    Plate {
                        plate: "UN1X".to_string(),
                        timestamp: 1000,
                    },
                    Ticket {
                        plate: "RE05BKG".to_string(),
                        road: 368,
                        mile1: 1234,
                        timestamp1: 1000000,
                        mile2: 1235,
                        timestamp2: 1000060,
                        speed: 6000,
                    },
                    WantHeartbeat { interval: 1243 },
                ],
            ],
        }];

        for test in tests {
            let mut fact = Factory::new();

            for (i, data) in test.test_data.iter().enumerate() {
                let messages = fact.push(data);

                assert!(messages.is_ok());
                let messages = messages.unwrap();
                assert_eq!(messages, test.expected[i]);
            }
        }
    }

    #[test]
    fn test_push_multiple() {
        use super::Message::*;
        struct Test<'a> {
            test_data: Vec<&'a [u8]>,
            expected: Vec<Vec<Message>>,
        }

        let tests = vec![Test {
            test_data: vec![&[
                0x80, 0x00, 0x42, 0x00, 0x64, 0x00, 0x3c, 0x80, 0x01, 0x70, 0x04, 0xd2, 0x00, 0x28,
            ]],
            expected: vec![vec![
                IAmCamera {
                    road: 66,
                    mile: 100,
                    limit: 60,
                },
                IAmCamera {
                    road: 368,
                    mile: 1234,
                    limit: 40,
                },
            ]],
        }];

        for test in tests {
            let mut fact = Factory::new();

            for (i, data) in test.test_data.iter().enumerate() {
                let messages = fact.push(data);

                assert!(messages.is_ok());
                let messages = messages.unwrap();
                assert_eq!(messages, test.expected[i]);
            }
        }
    }
}
