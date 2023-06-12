use super::i_am_dispatcher::IAmDispatcherFactory;
use super::plate::PlateFactory;
use super::want_heartbeat::WantHeartbeatFactory;
use super::{i_am_dispatcher, plate, want_heartbeat, ClientMessage};
use std::collections::VecDeque;

use super::i_am_camera;
use super::i_am_camera::IAmCameraFactory;

#[derive(Debug)]
pub enum ParsingErrorType {
    UnexpectedMessageType { id: String },
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ParsingError {
    error_type: ParsingErrorType,
}

pub struct ClientMessageFactory {
    buffer: VecDeque<u8>,
    curr_factory: Option<Box<dyn ClientMessageSubFactory + Send>>,
}

pub trait ClientMessageSubFactory {
    fn new() -> Self
    where
        Self: Sized;
    fn push(&mut self, data: u8) -> Option<ClientMessage>;
}

impl ClientMessageFactory {
    pub fn new() -> Self {
        ClientMessageFactory {
            buffer: VecDeque::new(),
            curr_factory: None,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Result<Vec<ClientMessage>, ParsingError> {
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

impl Default for ClientMessageFactory {
    fn default() -> Self {
        Self::new()
    }
}

fn new_sub_factory(id_byte: u8) -> Result<Box<dyn ClientMessageSubFactory + Send>, ParsingError> {
    match id_byte {
        plate::ID_BYTE => Ok(Box::new(PlateFactory::new())),
        want_heartbeat::ID_BYTE => Ok(Box::new(WantHeartbeatFactory::new())),
        i_am_camera::ID_BYTE => Ok(Box::new(IAmCameraFactory::new())),
        i_am_dispatcher::ID_BYTE => Ok(Box::new(IAmDispatcherFactory::new())),
        _ => Err(ParsingError {
            error_type: ParsingErrorType::UnexpectedMessageType {
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
        use super::ClientMessage::*;
        struct Test {
            test_data: Vec<u8>,
            expected: Vec<Vec<ClientMessage>>,
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
            let mut fact = ClientMessageFactory::new();

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
        use super::ClientMessage::*;
        struct Test<'a> {
            test_data: Vec<&'a [u8]>,
            expected: Vec<Vec<ClientMessage>>,
        }

        let tests = vec![Test {
            test_data: vec![
                &[0x80, 0x00, 0x42], //Starting IamCamera
                &[0x00, 0x64, 0x00],
                &[0x3c, 0x80, 0x01], //Finished IamCamera and starting new IAmCamera
                &[0x70, 0x04, 0xd2],
                &[0x00, 0x28],                         //Finished IamCamera
                &[0x20, 0x04, 0x55, 0x4e, 0x31, 0x58], //Start Plate
                &[
                    0x00, 0x00, 0x03, 0xe8, 0x40, 0x00, 0x00, 0x04, 0xdb, 0x81, 0x03, 0x00, 0x42,
                ], //Finish Plate,  Start and Finish WantHeartbeat, Start IAmDispatcjer
                &[0x01, 0x70, 0x13, 0x88],             // Finish IAmDispatcher
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
                Vec::new(),
                vec![
                    Plate {
                        plate: "UN1X".to_string(),
                        timestamp: 1000,
                    },
                    WantHeartbeat { interval: 1243 },
                ],
                vec![IAmDispatcher {
                    roads: vec![66, 368, 5000],
                }],
            ],
        }];

        for test in tests {
            let mut fact = ClientMessageFactory::new();

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
        use super::ClientMessage::*;
        struct Test<'a> {
            test_data: Vec<&'a [u8]>,
            expected: Vec<Vec<ClientMessage>>,
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
            let mut fact = ClientMessageFactory::new();

            for (i, data) in test.test_data.iter().enumerate() {
                let messages = fact.push(data);

                assert!(messages.is_ok());
                let messages = messages.unwrap();
                assert_eq!(messages, test.expected[i]);
            }
        }
    }
}
