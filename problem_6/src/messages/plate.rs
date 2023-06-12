use super::ClientMessage;
use super::ClientMessageSubFactory;

pub const ID_BYTE: u8 = 0x20;

pub struct PlateFactory {
    buffer: Vec<u8>,
    plate_len: usize,
    cursor: usize,
}

impl ClientMessageSubFactory for PlateFactory {
    fn new() -> Self {
        PlateFactory {
            buffer: Vec::new(),
            cursor: 0,
            plate_len: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<ClientMessage> {
        if self.buffer.is_empty() {
            self.buffer.extend(vec![0; data as usize + 4]);
            self.plate_len = data as usize;
        } else {
            self.buffer[self.cursor] = data;
            self.cursor += 1;
        }

        if self.cursor == self.buffer.len() {
            Some(ClientMessage::Plate {
                plate: String::from_utf8_lossy(&self.buffer[..self.plate_len]).to_string(),
                timestamp: u32::from_be_bytes(self.buffer[self.plate_len..].try_into().unwrap()),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::ClientMessage::Plate;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: Vec<u8>,
            expected: ClientMessage,
        }

        let tests = vec![
            Test {
                test_data: vec![0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x00, 0x03, 0xe8],
                expected: Plate {
                    plate: "UN1X".to_string(),
                    timestamp: 1000,
                },
            },
            Test {
                test_data: vec![
                    0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47, 0x00, 0x01, 0xe2, 0x40,
                ],
                expected: Plate {
                    plate: "RE05BKG".to_string(),
                    timestamp: 123456,
                },
            },
        ];

        for test in tests {
            let mut fact = PlateFactory::new();
            for &byte in test.test_data[..test.test_data.len() - 1].iter() {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(*test.test_data.last().unwrap());
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
