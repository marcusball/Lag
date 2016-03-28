use authoritative::AuthoritativeServer;
use std::io::Result;
//use std::io;
use std::io::prelude::*;
use mio::{Token, EventLoop, EventSet, PollOpt, TryRead};
use mio::tcp::{TcpStream};
//use byteorder::{ByteOrder, BigEndian, LittleEndian};

/// The state of the client's connection
pub enum ClientState{
    Connected,          // The TCP connection has been opened
    Hello,              // The client has sent an initial "hello message"
    Athenticated        // The client has successfully authenticated
}

pub struct GameClient{
    socket: TcpStream,
    token: Token,
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

    pub fn read(&mut self) -> Result<Option<u64>>{
        //println!("Fuck yeah!");

        // let mut buf = [0u8; 12];

        // let bytes_read = match self.socket.try_read(&mut buf){
            // Ok(None) => { return Ok(None); },
            // Ok(Some(n)) => n,
            // Err(e) => { return Err(e); }
        // };

        //let some_fucking_value = BigEndian::read_u16(buf[0..2].as_ref());

        //println!("Read {}", some_fucking_value);

        let mut recv_buf : Vec<u8> = Vec::with_capacity(8);

        let read_socket = <TcpStream as Read>::by_ref(&mut self.socket);

        // match read_socket.take(8).try_read_buf(&mut recv_buf){
        //     Ok(None) => {},
        //     Ok(Some(n)) => {},
        //     Err(e) => {}
        // };

        let mut total_bytes = 0;
        while let Some(b) = read_socket.take(8).try_read_buf(&mut recv_buf).ok(){
            match b{
                None => { break; },
                Some(bytes) => {
                    total_bytes += bytes;
                },
            };
        }

        recv_buf.truncate(total_bytes);

        //println!("Received: {:?}, {} bytes", recv_buf, total_bytes);

        let message : String = String::from_utf8(recv_buf).expect("Failed to read message!");

        println!("Received message: {}", message);

        Ok(None)
    }
}
