use authoritative::AuthoritativeServer;
use std::io::Result;
//use std::io;
use std::io::prelude::*;
use mio::{Token, EventLoop, EventSet, PollOpt};
use mio::tcp::{TcpStream};
//use byteorder::{ByteOrder, BigEndian, LittleEndian};

#[path="../shared/frame.rs"]
mod frame;
use frame::Message;

/// The state of the client's connection
pub enum ClientState{
    Connected,          // The TCP connection has been opened
    Hello,              // The client has sent an initial "hello message"
    Athenticated        // The client has successfully authenticated
}

pub struct GameClient{
    socket: TcpStream,
    pub token: Token,
    state: ClientState
}

impl GameClient{
    pub fn new(socket: TcpStream, token: Token) -> GameClient{
        GameClient {
            socket: socket,
            token: token,
            state: ClientState::Connected
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

    pub fn reregister(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>) -> Result<()>{
        println!("Reregistering token {:?}", self.token);

        event_loop.reregister(
            &self.socket,
            self.token,
            EventSet::readable(),
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),|{
            Ok(())
        }).or_else(|e|{
            println!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    pub fn read(&mut self) -> Result<Message>{
        // Create the socket from which we will read
        let read_socket = <TcpStream as Read>::by_ref(&mut self.socket);

        // Read the message from the socket
        let message = Message::read(read_socket);

        match message{
            Ok(message) => {
                match message{
                    Message::Text{message: ref message_text} => {
                        println!("Received message: {}", &message_text);
                    },
                    Message::Ping => {
                        println!("Received Ping!");
                    }
                }
                return Ok(message);
            },
            Err(e) => {
                println!("SHITS FUCKED UP! {:?}", e);
                return Err(e);
            }
        }
    }
}
