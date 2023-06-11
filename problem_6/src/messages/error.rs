use super::Message;
use super::MessageFactory;

pub const ID_BYTE: u8 = 0x10;

pub struct ErrorFactory {
    buffer: Vec<u8>,
    cursor: usize,
}

impl MessageFactory for ErrorFactory {
    fn new() -> Self {
        ErrorFactory {
            buffer: Vec::new(),
            cursor: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<Message> {
        if self.buffer.is_empty() {
            self.buffer.extend(vec![0; data as usize]);
        } else {
            self.buffer[self.cursor] = data;
            self.cursor += 1;
        }

        if self.cursor == self.buffer.len() {
            Some(Message::Error {
                msg: String::from_utf8_lossy(&self.buffer).to_string(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::Message::Error;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: Vec<u8>,
            expected: Message,
        }

        let tests = vec![
            Test {
                test_data: vec![0x03, 0x62, 0x61, 0x64],
                expected: Error {
                    msg: "bad".to_string(),
                },
            },
            Test {
                test_data: vec![
                    0x0b, 0x69, 0x6c, 0x6c, 0x65, 0x67, 0x61, 0x6c, 0x20, 0x6d, 0x73, 0x67,
                ],
                expected: Error {
                    msg: "illegal msg".to_string(),
                },
            },
        ];

        for test in tests {
            let mut fact = ErrorFactory::new();
            for &byte in test.test_data[..test.test_data.len() - 1].iter() {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(*test.test_data.last().unwrap());
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
