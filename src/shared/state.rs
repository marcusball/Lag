extern crate mio;
extern crate core;
use mio::util::Slab;
use mio::Token;
use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};

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

pub struct GameState{
    clients: Slab<ClientState>
}

impl GameState{
    pub fn new() -> GameState{
        GameState{
            clients: Slab::new_starting_at(Token(1), 128)
        }
    }
}
