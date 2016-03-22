extern crate mio;

use mio::tcp::*;
use mio::TryWrite;
use std::net::SocketAddr;
use std::io::Result;
use mio::{EventSet, EventLoop, Token, Handler, PollOpt};
//use std::thread;
//use std::thread::JoinHandle;
//use std::sync::{Arc, RwLock, Condvar};
//use std::sync::atomic::{AtomicBool, Ordering};

pub struct Client{
    socket: TcpStream,

    pub token: Token,

    interest: EventSet,

    send_queue: Vec<String>
}

impl Client{
    pub fn new(event_loop: &mut EventLoop<Client>, address: &SocketAddr) -> Result<Client>{
        let stream = TcpStream::connect(address);
        if stream.is_ok(){
            let mut client = Client {
                socket: stream.unwrap(),
                send_queue: Vec::new(),
                token: Token(0),
                interest: EventSet::readable()
            };

            client.register(event_loop).ok();

            return Ok(client);
        }
        else{
            Err(stream.unwrap_err())
        }
    }

    fn register(&mut self, event_loop: &mut EventLoop<Client>) -> Result<()>{
        event_loop.register(
            &self.socket,
            self.token,
            self.interest,
            PollOpt::edge()
        ).or_else(|e| {
            println!("Failed to register server! {:?}", e);
            Err(e)
        })
    }
}

impl Handler for Client{
    type Timeout = ();
    type Message = ();
}

#[test]
fn connect(){
    let addr = "127.0.0.1:6969".parse().unwrap();

    let mut event_loop = EventLoop::new().ok().expect("Failed to create event loop!");
    Client::new(&mut event_loop, &addr);
}

#[cfg(test)]
mod test {
    use mio::tcp::*;
    use mio::TryWrite;
    use std::net::SocketAddr;
    //use lag_client::Client;


}
