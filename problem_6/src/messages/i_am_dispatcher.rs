use super::ClientMessage;
use super::ClientMessageSubFactory;

pub const ID_BYTE: u8 = 0x81;

pub struct IAmDispatcherFactory {
    buffer: Vec<u8>,
    cursor: usize,
}

impl ClientMessageSubFactory for IAmDispatcherFactory {
    fn new() -> Self {
        IAmDispatcherFactory {
            buffer: Vec::new(),
            cursor: 0,
        }
    }

    fn push(&mut self, data: u8) -> Option<ClientMessage> {
        if self.buffer.is_empty() {
            self.buffer.extend(vec![0; 2 * data as usize]);
        } else {
            self.buffer[self.cursor] = data;
            self.cursor += 1;
        }

        if self.cursor == self.buffer.len() {
            let mut roads = Vec::with_capacity(self.buffer.len() >> 2);
            for road in self.buffer.chunks(2) {
                roads.push(u16::from_be_bytes(road.try_into().unwrap()));
            }
            Some(ClientMessage::IAmDispatcher { roads })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::ClientMessage::IAmDispatcher;
    use super::*;

    #[test]
    fn test_push() {
        struct Test {
            test_data: Vec<u8>,
            expected: ClientMessage,
        }

        let tests = vec![
            Test {
                test_data: vec![0x01, 0x00, 0x42],
                expected: IAmDispatcher { roads: vec![66] },
            },
            Test {
                test_data: vec![0x03, 0x00, 0x42, 0x01, 0x70, 0x13, 0x88],
                expected: IAmDispatcher {
                    roads: vec![66, 368, 5000],
                },
            },
        ];

        for test in tests {
            let mut fact = IAmDispatcherFactory::new();
            for &byte in test.test_data[..test.test_data.len() - 1].iter() {
                assert!(fact.push(byte).is_none());
            }

            let msg = fact.push(*test.test_data.last().unwrap());
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), test.expected);
        }
    }
}
