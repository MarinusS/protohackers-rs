pub const ID_BYTE: u8 = 0x41;

#[derive(Debug, PartialEq)]
pub struct Hearbeat;

impl Hearbeat {
    fn encode(&self) -> Vec<u8> {
        let mut data = vec![ID_BYTE];
        data
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_encode() {
        let msg = Hearbeat;
        assert_eq!(msg.encode(), vec![ID_BYTE]);
    }
}
