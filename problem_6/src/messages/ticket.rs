pub const ID_BYTE: u8 = 0x21;

#[derive(Debug, PartialEq)]
pub struct Ticket {
    plate: String,
    road: u16,
    mile1: u16,
    timestamp1: u32,
    mile2: u16,
    timestamp2: u32,
    speed: u16,
}

impl Ticket {
    fn encode(&self) -> Vec<u8> {
        let plate_len = self.plate.as_bytes().len();
        let mut data = Vec::with_capacity(1 + 1 + plate_len + 2 + 2 + 4 + 2 + 4 + 2);

        data.push(ID_BYTE);
        data.push(plate_len as u8);
        data.extend_from_slice(self.plate.as_bytes());
        data.extend_from_slice(&self.road.to_be_bytes());
        data.extend_from_slice(&self.mile1.to_be_bytes());
        data.extend_from_slice(&self.timestamp1.to_be_bytes());
        data.extend_from_slice(&self.mile2.to_be_bytes());
        data.extend_from_slice(&self.timestamp2.to_be_bytes());
        data.extend_from_slice(&self.speed.to_be_bytes());

        data
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_encode() {
        struct Test {
            test_data: Ticket,
            expected: Vec<u8>,
        }

        let tests = vec![
            Test {
                test_data: Ticket {
                    plate: "UN1X".to_string(),
                    road: 66,
                    mile1: 100,
                    timestamp1: 123456,
                    mile2: 110,
                    timestamp2: 123816,
                    speed: 10000,
                },
                expected: vec![
                    0x21, 0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x42, 0x00, 0x64, 0x00, 0x01, 0xe2,
                    0x40, 0x00, 0x6e, 0x00, 0x01, 0xe3, 0xa8, 0x27, 0x10,
                ],
            },
            Test {
                test_data: Ticket {
                    plate: "RE05BKG".to_string(),
                    road: 368,
                    mile1: 1234,
                    timestamp1: 1000000,
                    mile2: 1235,
                    timestamp2: 1000060,
                    speed: 6000,
                },
                expected: vec![
                    0x21, 0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47, 0x01, 0x70, 0x04, 0xd2,
                    0x00, 0x0f, 0x42, 0x40, 0x04, 0xd3, 0x00, 0x0f, 0x42, 0x7c, 0x17, 0x70,
                ],
            },
        ];

        for test in tests {
            assert_eq!(test.test_data.encode(), test.expected);
        }
    }
}
