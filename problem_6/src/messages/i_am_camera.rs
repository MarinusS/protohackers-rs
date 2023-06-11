use super::Message;
use super::MessageFactory;

pub const ID_BYTE: u8 = 0x80;

pub struct IAmCameraFactory {
    buffer: [u8; 6],
    cursor: usize,
}

impl MessageFactory for IAmCameraFactory {
    fn new() -> Self {
        IAmCameraFactory {
            buffer: [0; 6],
            cursor: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<Message> {
        self.buffer[self.cursor] = data;
        self.cursor += 1;

        if self.cursor == self.buffer.len() {
            Some(Message::IAmCamera {
                road: u16::from_be_bytes(self.buffer[0..2].try_into().unwrap()),
                mile: u16::from_be_bytes(self.buffer[2..4].try_into().unwrap()),
                limit: u16::from_be_bytes(self.buffer[4..6].try_into().unwrap()),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::Message::IAmCamera;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: [u8; 6],
            expected: Message,
        }

        let tests = vec![
            Test {
                test_data: [0x00, 0x42, 0x00, 0x64, 0x00, 0x3c],
                expected: IAmCamera {
                    road: 66,
                    mile: 100,
                    limit: 60,
                },
            },
            Test {
                test_data: [0x01, 0x70, 0x04, 0xd2, 0x00, 0x28],
                expected: IAmCamera {
                    road: 368,
                    mile: 1234,
                    limit: 40,
                },
            },
        ];

        for test in tests {
            let mut fact = IAmCameraFactory::new();
            for &byte in test.test_data.iter().take(5) {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(test.test_data[5]);
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
