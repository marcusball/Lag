extern crate mio;
extern crate core;
use mio::util::Slab;
use mio::Token;
use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::{Read, ErrorKind, Result, Error};
use byteorder::{ByteOrder, BigEndian};

#[derive(Copy, Clone, Debug)]
pub struct ClientState{
    /// The client ID
    pub id: u32,

    /// The world location of this client
    pub position: (i32, i32, i32),

    /// The rotational yaw of this client
    pub rotation: i32
}

impl ClientState{
    pub fn new(id: u32) -> ClientState{
        ClientState{
            id: id,
            position: (0i32, 0i32, 0i32),
            rotation: 0i32
        }
    }

    pub fn read<R: Read>(input: &mut R) -> Result<ClientState>{
        // The number of bytes we're expecting to read
        const buffer_length : usize = 20;

        let mut message_buf = [0u8; buffer_length];
        let bytes_read = try!(input.read(&mut message_buf));

        // Error checking; Make sure we read bytes of the message,
        // and ensure it's the length the client claimed it would be.
        if bytes_read != buffer_length{
            return Err(Error::new(ErrorKind::Other, format!("Expected string of {} bytes, received a string of {} bytes!", buffer_length, bytes_read)));
        }

        let mut client_state = ClientState{ id: 0, position: (0i32, 0i32, 0i32), rotation: 0i32 };

        client_state.id         = BigEndian::read_u32(&message_buf[00..04]);
        client_state.position.0 = BigEndian::read_i32(&message_buf[04..08]);
        client_state.position.1 = BigEndian::read_i32(&message_buf[08..12]);
        client_state.position.2 = BigEndian::read_i32(&message_buf[12..16]);
        client_state.rotation   = BigEndian::read_i32(&message_buf[16..20]);

        return Ok(client_state);
    }
}

impl PartialEq for ClientState{
    fn eq(&self, other: &ClientState) -> bool{
        self.id == other.id
    }
}

impl Hash for ClientState{
    fn hash<H: Hasher>(&self, state: &mut H){
        state.write_u32(self.id);
        state.finish();
    }
}

#[derive(Clone, Debug)]
pub struct GameState{
    clients: HashMap<u32, ClientState>
}

impl GameState{
    pub fn new() -> GameState{
        GameState{
            clients: HashMap::with_capacity(32)
        }
    }
}
