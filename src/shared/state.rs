extern crate mio;
use mio::util::Slab;
use mio::Token;

pub struct ClientState{
    /// The client ID
    pub id: u32,

    /// The world location of this client
    pub position: (f32, f32, f32),

    /// The rotational yaw of this client
    pub rotation: f32
}

impl ClientState{
    pub fn new(id: u32) -> ClientState{
        ClientState{
            id: id,
            position: (0f32, 0f32, 0f32),
            rotation: 0f32
        }
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
