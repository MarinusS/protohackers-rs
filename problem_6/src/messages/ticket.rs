use super::ClientMessage;
use super::ClientMessageSubFactory;

pub const ID_BYTE: u8 = 0x21;

pub struct TicketFactory {
    buffer: Vec<u8>,
    plate_len: usize,
    cursor: usize,
}

impl ClientMessageSubFactory for TicketFactory {
    fn new() -> Self {
        TicketFactory {
            buffer: Vec::new(),
            cursor: 0,
            plate_len: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<ClientMessage> {
        if self.buffer.is_empty() {
            self.buffer.extend(vec![0; data as usize + 16]);
            self.plate_len = data as usize;
        } else {
            self.buffer[self.cursor] = data;
            self.cursor += 1;
        }

        if self.cursor == self.buffer.len() {
            let road_offset = self.plate_len;
            let mile1_offset = road_offset + 2;
            let timestamp1_offset = mile1_offset + 2;
            let mile2_offset = timestamp1_offset + 4;
            let timestamp2_offset = mile2_offset + 2;
            let speed_offset = timestamp2_offset + 4;

            Some(ClientMessage::Ticket {
                plate: String::from_utf8_lossy(&self.buffer[..road_offset]).to_string(),
                road: u16::from_be_bytes(
                    self.buffer[road_offset..mile1_offset].try_into().unwrap(),
                ),
                mile1: u16::from_be_bytes(
                    self.buffer[mile1_offset..timestamp1_offset]
                        .try_into()
                        .unwrap(),
                ),
                timestamp1: u32::from_be_bytes(
                    self.buffer[timestamp1_offset..mile2_offset]
                        .try_into()
                        .unwrap(),
                ),
                mile2: u16::from_be_bytes(
                    self.buffer[mile2_offset..timestamp2_offset]
                        .try_into()
                        .unwrap(),
                ),
                timestamp2: u32::from_be_bytes(
                    self.buffer[timestamp2_offset..speed_offset]
                        .try_into()
                        .unwrap(),
                ),
                speed: u16::from_be_bytes(self.buffer[speed_offset..].try_into().unwrap()),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::ClientMessage::Ticket;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: Vec<u8>,
            expected: ClientMessage,
        }

        let tests = vec![
            Test {
                test_data: vec![
                    0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x42, 0x00, 0x64, 0x00, 0x01, 0xe2, 0x40,
                    0x00, 0x6e, 0x00, 0x01, 0xe3, 0xa8, 0x27, 0x10,
                ],
                expected: Ticket {
                    plate: "UN1X".to_string(),
                    road: 66,
                    mile1: 100,
                    timestamp1: 123456,
                    mile2: 110,
                    timestamp2: 123816,
                    speed: 10000,
                },
            },
            Test {
                test_data: vec![
                    0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47, 0x01, 0x70, 0x04, 0xd2, 0x00,
                    0x0f, 0x42, 0x40, 0x04, 0xd3, 0x00, 0x0f, 0x42, 0x7c, 0x17, 0x70,
                ],
                expected: Ticket {
                    plate: "RE05BKG".to_string(),
                    road: 368,
                    mile1: 1234,
                    timestamp1: 1000000,
                    mile2: 1235,
                    timestamp2: 1000060,
                    speed: 6000,
                },
            },
        ];

        for test in tests {
            let mut fact = TicketFactory::new();
            for &byte in test.test_data[..test.test_data.len() - 1].iter() {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(*test.test_data.last().unwrap());
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
