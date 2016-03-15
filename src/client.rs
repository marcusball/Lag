use authoritative::AuthoritativeServer;
use std::io::Result;
use mio::{Token, EventLoop, EventSet, PollOpt};
use mio::tcp::TcpStream;

pub struct GameClient{
    socket: TcpStream,
    token: Token
}

impl GameClient{
    pub fn new(socket: TcpStream, token: Token) -> GameClient{
        GameClient {
            socket: socket,
            token: token,
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>) -> Result<()>{
        println!("Registering token {:?}", self.token);

        event_loop.register(
            &self.socket,
            self.token,
            EventSet::readable(),
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),|{
            Ok(())
        }).or_else(|e|{
            println!("Failed to register {:?}, {:?}", self.token, e);
            Err(e)
        })
    }
}
