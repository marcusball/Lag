extern crate log;
extern crate mio;
extern crate core;
use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::{Read, ErrorKind, Result, Error};
use byteorder::{ByteOrder, BigEndian};
use std::mem;
use std::ops::Add;

#[derive(Copy, Debug, Clone)]
pub struct Position(pub i32, pub i32, pub i32);
#[derive(Copy, Debug, Clone)]
pub struct Rotation(pub i32);
#[derive(Copy, Debug, Clone)]
pub struct Transform{pub position: Position, pub rotation: Rotation}

impl Position{
    pub fn zero() -> Position{ Position(0,0,0) }
}

impl Add for Position{
    type Output = Position;

    fn add(self, other: Position) -> Position {
        Position(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

impl Rotation{
    pub fn zero() -> Rotation{ Rotation(0) }
}

impl Transform{
    pub fn new() -> Transform{ Self::from_components(Position::zero(), Rotation::zero()) }
    pub fn from_position(position: Position) -> Transform{ Self::from_components(position, Rotation::zero()) }
    pub fn from_rotation(rotation: Rotation) -> Transform{ Self::from_components(Position::zero(), rotation) }
    pub fn from_components(position: Position, rotation: Rotation) -> Transform { Transform{ position: position, rotation: rotation } }
}

#[derive(Copy, Clone, Debug)]
pub struct ClientState{
    /// The client ID
    pub id: u32,

    /// The world location of this client
    pub position: Position,

    /// The rotational yaw of this client
    pub rotation: Rotation
}

impl ClientState{
    pub fn new(id: u32) -> ClientState{
        ClientState{
            id: id,
            position: Position::zero(),
            rotation: Rotation::zero()
        }
    }

    pub fn read<R: Read>(input: &mut R) -> Result<ClientState>{
        // The number of bytes we're expecting to read
        const BUFFER_LENGTH : usize = 20;

        assert_eq!(BUFFER_LENGTH, mem::size_of::<Self>());

        let mut message_buf = [0u8; BUFFER_LENGTH];
        let bytes_read = try!(input.read(&mut message_buf));

        // Error checking; Make sure we read bytes of the message,
        // and ensure it's the length the client claimed it would be.
        if bytes_read != BUFFER_LENGTH{
            return Err(Error::new(ErrorKind::Other, format!("Expected string of {} bytes, received a string of {} bytes!", BUFFER_LENGTH, bytes_read)));
        }

        let mut client_state = ClientState::new(0);

        client_state.id         = BigEndian::read_u32(&message_buf[00..04]);
        client_state.position.0 = BigEndian::read_i32(&message_buf[04..08]);
        client_state.position.1 = BigEndian::read_i32(&message_buf[08..12]);
        client_state.position.2 = BigEndian::read_i32(&message_buf[12..16]);
        client_state.rotation.0 = BigEndian::read_i32(&message_buf[16..20]);

        return Ok(client_state);
    }

    pub fn to_bytes(&self) -> Vec<u8>{
        let mut buf = [0u8; 20];

        BigEndian::write_u32(&mut buf[00..04], self.id);
        BigEndian::write_i32(&mut buf[04..08], self.position.0);
        BigEndian::write_i32(&mut buf[08..12], self.position.1);
        BigEndian::write_i32(&mut buf[12..16], self.position.2);
        BigEndian::write_i32(&mut buf[16..20], self.rotation.0);

        return buf.to_vec();
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
    pub clients: HashMap<u32, ClientState>
}

impl GameState{
    pub fn new() -> GameState{
        GameState{
            clients: HashMap::with_capacity(32)
        }
    }

    pub fn update_from_vec(&mut self, update_vec: &Vec<ClientState>){
        self.clients.clear();
        for client in update_vec{
            self.clients.insert(client.id, *client);
        }
    }
}
