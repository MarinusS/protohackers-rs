pub const ID_BYTE: u8 = 0x10;

#[derive(Debug, PartialEq)]
pub struct Error {
    msg: String,
}

impl Error {
    fn encode(&self) -> Vec<u8> {
        let msg_field_len = self.msg.as_bytes().len();
        let mut data = Vec::with_capacity(2 + msg_field_len);

        data.push(ID_BYTE);
        data.push(msg_field_len as u8);
        data.extend_from_slice(self.msg.as_bytes());

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
            test_data: Error,
            expected: Vec<u8>,
        }

        let tests = vec![
            Test {
                test_data: Error {
                    msg: "bad".to_string(),
                },
                expected: vec![0x10, 0x03, 0x62, 0x61, 0x64],
            },
            Test {
                test_data: Error {
                    msg: "illegal msg".to_string(),
                },
                expected: vec![
                    0x10, 0x0b, 0x69, 0x6c, 0x6c, 0x65, 0x67, 0x61, 0x6c, 0x20, 0x6d, 0x73, 0x67,
                ],
            },
        ];

        for test in tests {
            assert_eq!(test.test_data.encode(), test.expected);
        }
    }
}
