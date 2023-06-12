pub const ID_BYTE: u8 = 0x41;

#[derive(Debug, PartialEq)]
pub struct Heartbeat;

impl Heartbeat {
    pub fn encode() -> Vec<u8> {
        let data = vec![ID_BYTE];
        data
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(Heartbeat::encode(), vec![ID_BYTE]);
    }
}
