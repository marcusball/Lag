extern crate byteorder;
use std::io::{Read, ErrorKind, Result, Error};
use byteorder::{ByteOrder, BigEndian};

#[path="../shared/state.rs"]
mod state;
use state::{ClientState, GameState};

const MAGIC_BYTES: u32 = 0x4C414721; // b'LAG!'

#[derive(Debug, PartialEq, Clone)]
pub enum MessageCode{
    Text            = 0x01,
    ClientUpdate    = 0x02,
    GameStateUpdate = 0x03,
    Ping            = 0xFF
}

impl MessageCode{
    pub fn from_u8(byte: u8) -> Option<MessageCode>{
        match byte{
            0x01 => { Some(MessageCode::Text) },
            0x02 => { Some(MessageCode::ClientUpdate) },
            0x03 => { Some(MessageCode::GameStateUpdate) },
            0xFF => { Some(MessageCode::Ping) },
            _    => { None }
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
        println!("begin messageheader::read");
        let mut header_buf = [0u8; 9];
        let header_buf_length = match input.read(&mut header_buf){
            Ok(n) => { n },
            Err(e) => { return Err(e); }
        };

        if header_buf_length < 9{
            return Err(Error::new(ErrorKind::Other, format!("Message header Input length ({}) is not long enough!", header_buf_length)));
        }

        return Self::read_slice(header_buf.as_ref());
    }

    pub fn read_slice(input: &[u8]) -> Result<MessageHeader>{
        if input.len() < 8{
            return Err(Error::new(ErrorKind::Other, format!("Message header slice Input length ({}) is not long enough!", input.len())));
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



#[derive(Hash, Debug, Clone)]
pub enum Message{
    Ping,
    Text{ message: String },
    ClientUpdate (ClientState),
    GameStateUpdate (Vec<ClientState>)
}

impl Message{
    pub fn new_text_message(msg: String) -> Message{
        Message::Text{ message: msg}
    }

    pub fn new_client_update_message(client_state: &ClientState) -> Message{
        Message::ClientUpdate( *client_state )
    }

    /// Read bytes from the input parameter, and return a parsed Message.
    pub fn read<R: Read>(mut input: &mut R) -> Result<Message>{
        println!("Begin message::read");
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
            },
            MessageCode::ClientUpdate => {
                println!("Reading client update data!");
                Self::read_client_update_message(&mut input, &header)
            },
            MessageCode::GameStateUpdate => {
                println!("Received game state update");
                panic!();
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

    fn read_client_update_message<R: Read>(input: &mut R, header: &MessageHeader) -> Result<Message>{
        let client_state = try!(ClientState::read(input));

        return Ok(Message::ClientUpdate(client_state));
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
            },
            &Message::ClientUpdate(ref client_state) =>{
                let mut buf = [0u8; 20];

                BigEndian::write_u32(&mut buf[00..04], client_state.id);
                BigEndian::write_i32(&mut buf[04..08], client_state.position.0);
                BigEndian::write_i32(&mut buf[08..12], client_state.position.1);
                BigEndian::write_i32(&mut buf[12..16], client_state.position.2);
                BigEndian::write_i32(&mut buf[16..20], client_state.rotation);

                return buf.to_vec();
            },
            &Message::GameStateUpdate( _ ) => {
                panic!();
                return vec![];
            }
        }
    }

    fn get_message_code(&self) -> MessageCode{
        match self{
            &Message::Text{message: _} => { return MessageCode::Text; },
            &Message::Ping => { return MessageCode::Ping; },
            &Message::ClientUpdate(_) => { return MessageCode::ClientUpdate; },
            &Message::GameStateUpdate(_) => { return MessageCode::GameStateUpdate; }
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
    use state::ClientState;
    use byteorder::{ByteOrder, BigEndian};

    #[test]
    fn test_read_header(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageHeader::read_slice(test_message.as_slice()).unwrap();

        assert_eq!(BigEndian::read_u32(b"LAG!"), message.magic);
        assert_eq!(3, message.length);
    }

    #[test]
    fn test_is_valid(){
        // Send the message `LAG!0003`
        let test_message = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 3u8];
        let message = MessageHeader::read_slice(test_message.as_slice()).unwrap();

        assert!(message.is_valid());
    }

    #[test]
    fn test_text_message_to_frame(){
        let test_message = Message::new_text_message(String::from("Test"));

        let frame = test_message.to_frame();

        assert_eq!(frame.payload, String::from("Test").into_bytes());
    }

    #[test]
    fn test_message_to_bytes(){
        let test_message_output = vec!['L' as u8, 'A' as u8, 'G' as u8, '!' as u8, 1u8, 0u8, 0u8, 0u8, 4u8, 'T' as u8, 'e' as u8, 's' as u8, 't' as u8];
        let test_message = Message::new_text_message(String::from("Test"));

        assert_eq!(test_message.to_frame().to_bytes(), test_message_output);
    }

    #[test]
    fn test_client_update_serialize(){
        let mut test_client = ClientState::new(1);
        test_client.position = (1,2,3);
        let god = Message::new_client_update_message(&test_client);
        let fucking = god.to_frame();
        let damn = fucking.to_bytes();
        let mut motherfucker = damn.clone();
        let mut shit = motherfucker.as_slice();
        let mut fuck = shit;//.as_mut();
        let mut das_thing = fuck.as_ref();

        let deserialized_message = Message::read(&mut das_thing).unwrap();

        match deserialized_message{
            Message::ClientUpdate(client_state) => {
                assert_eq!(client_state.position, test_client.position);
            },
            _ => { panic!(); }
        }
    }
}
