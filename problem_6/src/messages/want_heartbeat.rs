use super::Message;
use super::MessageFactory;

pub const ID_BYTE: u8 = 0x40;

pub struct WantHeartbeatFactory {
    buffer: [u8; 4],
    cursor: usize,
}

impl MessageFactory for WantHeartbeatFactory {
    fn new() -> Self {
        WantHeartbeatFactory {
            buffer: [0; 4],
            cursor: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<Message> {
        self.buffer[self.cursor] = data;
        self.cursor += 1;

        if self.cursor == self.buffer.len() {
            Some(Message::WantHeartbeat {
                interval: u32::from_be_bytes(self.buffer),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::Message::WantHeartbeat;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: [u8; 4],
            expected: Message,
        }

        let tests = vec![
            Test {
                test_data: [0x00, 0x00, 0x00, 0x0a],
                expected: WantHeartbeat { interval: 10 },
            },
            Test {
                test_data: [0x00, 0x00, 0x04, 0xdb],
                expected: WantHeartbeat { interval: 1243 },
            },
        ];

        for test in tests {
            let mut fact = WantHeartbeatFactory::new();
            for &byte in test.test_data[..test.test_data.len() - 1].iter() {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(*test.test_data.last().unwrap());
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
