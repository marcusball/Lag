extern crate byteorder;
use std::io::{Read, ErrorKind, Result, Error};
use byteorder::{ByteOrder, BigEndian};

const MAGIC_BYTES: u32 = 0x4C414721; // b'LAG!'

#[derive(Debug, PartialEq, Clone)]
pub enum MessageCode{
    Text = 0x01,
    Ping = 0xFF
}

impl MessageCode{
    pub fn from_u8(byte: u8) -> Option<MessageCode>{
        match byte{
            1 => { Some(MessageCode::Text) },
            0xFF => { Some(MessageCode::Ping) },
            _ => { None }
        }
    }
}

pub struct MessageHeader{
    /// Magic number starting each frame, should be b"lag!"
    pub magic: u32,

    /// The type of message contained in this frame
    pub code: MessageCode,

    /// The length of the payload
    pub length: u32
}


impl MessageHeader{
    pub fn new(code: MessageCode, data_len: u32) -> MessageHeader{
        MessageHeader{
            magic: MAGIC_BYTES,
            code: code,
            length: data_len
        }
    }

    pub fn read<R: Read>(input: &mut R) -> Result<MessageHeader>{
        let mut header_buf = [0u8; 9];
        let header_buf_length = match input.read(&mut header_buf){
            Ok(n) => { n },
            Err(e) => { return Err(e); }
        };

        if header_buf_length < 9{
            return Err(Error::new(ErrorKind::Other, "Input length is not long enough!"));
        }

        return Self::read_slice(header_buf.as_ref());
    }

    pub fn read_slice(input: &[u8]) -> Result<MessageHeader>{
        if input.len() < 8{
            return Err(Error::new(ErrorKind::Other, "Input length is not long enough!"));
        }

        let message_code_byte = input[4..5].first().unwrap();
        let message_code = MessageCode::from_u8(*message_code_byte);
        if message_code.is_none(){
            return Err(Error::new(ErrorKind::Other, format!("Received unknown message code {:x}!", message_code_byte)));
        }

        let payload_length = BigEndian::read_u32(input[5..9].as_ref());

        Ok(MessageHeader{
            magic: BigEndian::read_u32(input[0..4].as_ref()),
            code: message_code.unwrap(),
            length: payload_length,
        })
    }

    pub fn is_valid(&self) -> bool{
        self.magic == MAGIC_BYTES
    }


    pub fn to_bytes(&self) -> Vec<u8>{
        let mut buffer = [0u8; 4 + 1 + 4];

        BigEndian::write_u32(&mut buffer[0..4], self.magic);
        buffer[4] = self.code.clone() as u8;
        BigEndian::write_u32(&mut buffer[5..9], self.length);

        return buffer.to_vec();
    }
}



#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Message{
    Ping,
    Text{ message: String }
}

impl Message{
    pub fn new_text_message(msg: String) -> Message{
        Message::Text{ message: msg}
    }

    /// Read bytes from the input parameter, and return a parsed Message.
    pub fn read<R: Read>(mut input: &mut R) -> Result<Message>{
        let header = MessageHeader::read(&mut input);
        if header.is_err(){
            return Err(header.err().unwrap());
        }

        let header = header.unwrap();
        if !header.is_valid(){
            return Err(Error::new(ErrorKind::InvalidInput, String::from("Received an invalid message header!")));
        }

        let message = match header.code{
            MessageCode::Text => {
                Self::read_text_message(&mut input, &header)
                //return Ok(Message::Text{message: String::from("Hello!")});
            },
            MessageCode::Ping => {
                Ok(Message::Ping)
            }
            //_ => { return Err(Error::new(ErrorKind::InvalidInput, format!("Received an unhandled message type, {:?}!", header.code))); }
        };

        return message;
    }


    fn read_text_message<R: Read>(input: &mut R, header: &MessageHeader) -> Result<Message>{
        let mut message_buf = [0u8; 1024];
        let bytes_read = input.read(&mut message_buf);
        // Error checking; Make sure we read bytes of the message,
        // and ensure it's the length the client claimed it would be.
        match bytes_read{
            Ok(bytes_read) => {
                if bytes_read != header.length as usize{
                    return Err(Error::new(ErrorKind::Other, format!("Expected string of {} bytes, received a string of {} bytes!", header.length, bytes_read)));
                }
            },
            Err(e) => {
                return Err(e);
            }
        }

        let message = String::from_utf8(message_buf.iter().take(bytes_read.unwrap()).map(|br| *br).collect::<Vec<u8>>()).unwrap();

        // If all checks passed, return the message.
        Ok(Message::Text{message: message})
    }


    pub fn to_bytes(&self) -> Vec<u8>{
        match self{
            &Message::Text{ref message} => {
                let message_copy = message.clone();
                //data_buf.append(&mut message_copy.into_bytes());
                return message_copy.into_bytes();
            },
            &Message::Ping => {
                return MessageHeader::new(MessageCode::Ping, 0u32).to_bytes();
            }
        }
    }

    fn get_message_code(&self) -> MessageCode{
        match self{
            &Message::Text{message: _} => { return MessageCode::Text; },
            &Message::Ping => { return MessageCode::Ping; }
        }
    }
}

impl ToFrame for Message{
    fn to_frame(&self) -> MessageFrame{
        let payload = self.to_bytes();
        MessageFrame{
            header: MessageHeader::new(self.get_message_code(), payload.len() as u32),
            payload: payload
        }
    }
}


pub trait ToFrame{
    /// Create a MessageFrame struct from the given data
    fn to_frame(&self) -> MessageFrame;
}


pub struct MessageFrame{
    header: MessageHeader,

    payload: Vec<u8>
}

impl MessageFrame{
    pub fn to_bytes(&self) -> Vec<u8>{
        let mut header_bytes = self.header.to_bytes();
        let mut bytes = Vec::with_capacity(header_bytes.len() + self.header.length as usize);
        let mut payload_bytes = self.payload.clone();
        bytes.append(&mut header_bytes);
        bytes.append(&mut payload_bytes);
        return bytes;
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
        let message = MessageHeader::read_slice(test_message.as_slice()).unwrap();

        assert_eq!(BigEndian::read_u32(b"LAG!"), message.magic);
        assert_eq!(3, message.length);
    }

    #[test]
    fn testIsValid(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageHeader::read_slice(test_message.as_slice()).unwrap();

        assert!(message.is_valid());
    }

    #[test]
    fn testTextMessageToFrame(){
        let test_message = Message::new_text_message(String::from("Test"));

        let frame = test_message.to_frame();

        assert_eq!(frame.payload, String::from("Test").into_bytes());
    }

    #[test]
    fn testMessageToBytes(){
        let test_message_output = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 4u8, 'T' as u8, 'e' as u8, 's' as u8, 't' as u8];
        let test_message = Message::new_text_message(String::from("Test"));

        assert_eq!(test_message.to_frame().to_bytes(), test_message_output);
    }
}
