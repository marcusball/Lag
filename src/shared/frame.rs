extern crate num;
extern crate enum_primitive;

use std::io::{Read, Write, ErrorKind, Cursor, Result, Error};
use byteorder::{ByteOrder, BigEndian};
use num::FromPrimitive;

const MAGIC_BYTES: u32 = 0x4C414721; // b'LAG!'

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    pub enum MessageCode{
        Text = 0x01
    }
}

pub struct MessageFrame{
    /// Magic number starting each frame, should be b"lag!"
    pub magic: u32,

    /// The type of message contained in this frame
    pub code: MessageCode,

    /// The length of the payload
    pub length: u32,

    /// The payload of the message
    pub payload: Vec<u8>
}


impl MessageFrame{
    pub fn new(code: MessageCode, data: Vec<u8>) -> MessageFrame{
        MessageFrame{
            magic: MAGIC_BYTES,
            code: code,
            length: data.len() as u32,
            payload: data
        }
    }

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

        let message_code_byte = input[4..5].first().unwrap();
        let message_code = MessageCode::from_u8(*message_code_byte);
        if message_code.is_none(){
            return Err(Error::new(ErrorKind::Other, format!("Received unknown message code {:x}!", message_code_byte)));
        }

        Ok(MessageFrame{
            magic: BigEndian::read_u32(input[0..4].as_ref()),
            code: message_code.unwrap(),
            length: BigEndian::read_u32(input[5..9].as_ref()),
            payload: vec![1,2,3]
        })
    }

    pub fn is_valid(&self) -> bool{
        self.magic == MAGIC_BYTES
    }
}

pub trait ToFrame{
    /// Create a MessageFrame struct from the given data
    fn to_frame(&self) -> MessageFrame;
}


pub struct TextMessage{
    pub message: String
}

impl TextMessage{
    /// Create a new chat text message packet
    pub fn new(message: String) -> TextMessage{
        TextMessage{ message: message}
    }
}

impl ToFrame for TextMessage{
    fn to_frame(&self) -> MessageFrame{
        let message = self.message.clone();
        MessageFrame::new(MessageCode::Text, message.into_bytes())
    }
}


#[cfg(test)]
mod test{
    use super::*;
    use byteorder::{ByteOrder, BigEndian};

    #[test]
    fn testReadHeader(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageFrame::read_slice(test_message.as_slice()).unwrap();

        assert_eq!(BigEndian::read_u32(b"LAG!"), message.magic);
        assert_eq!(3, message.length);
    }

    #[test]
    fn testIsValid(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageFrame::read_slice(test_message.as_slice()).unwrap();

        assert!(message.is_valid());
    }

    #[test]
    fn testTextMessageToFrame(){
        let test_message = TextMessage::new(String::from("Test"));

        let frame = test_message.to_frame();

        assert_eq!(frame.payload, String::from("Test").into_bytes());
    }
}
