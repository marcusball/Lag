use std::io::{Read, Write, ErrorKind, Cursor, Result, Error};
use byteorder::{ByteOrder, BigEndian};

const MAGIC_BYTES: u32 = 0x4C414721; // b'LAG!'

pub struct MessageFrame{
    /// Magic number starting each frame, should be b"lag!"
    pub magic: u32,

    /// The length of the payload
    pub length: u32,

    /// The payload of the message
    pub payload: Vec<u8>
}


impl MessageFrame{
    pub fn read<R: Read>(input: &mut R) -> Result<MessageFrame>{
        let mut header_buf = [0u8; 8];
        let header_buf_length = match input.read(&mut header_buf){
            Ok(n) => { n },
            Err(e) => { return Err(e); }
        };

        if header_buf_length < 8{
            return Err(Error::new(ErrorKind::Other, "Input length is not long enough!"));
        }

        return Self::read_slice(header_buf.as_ref());
    }

    pub fn read_slice(input: &[u8]) -> Result<MessageFrame>{
        if input.len() < 8{
            return Err(Error::new(ErrorKind::Other, "Input length is not long enough!"));
        }

        Ok(MessageFrame{
            magic: BigEndian::read_u32(input[0..4].as_ref()),
            length: BigEndian::read_u32(input[4..8].as_ref()),
            payload: vec![1,2,3]
        })
    }

    pub fn is_valid(&self) -> bool{
        self.magic == MAGIC_BYTES
    }
}

#[cfg(test)]
mod test{
    use super::MessageFrame;
    use byteorder::{ByteOrder, BigEndian};

    #[test]
    fn testReadHeader(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageFrame::read_slice(test_message.as_slice()).unwrap();

        assert_eq!(BigEndian::read_u32(b"LAG!"), message.magic);
        assert_eq!(3, message.length);
    }

    #[test]
    fn testIsValid(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageFrame::read_slice(test_message.as_slice()).unwrap();

        assert!(message.is_valid());
    }
}
