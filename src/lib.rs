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
use std::sync::atomic::{AtomicUsize, Ordering};

const CLIENT_TOKEN: mio::Token = mio::Token(1);

pub struct Client{
    socket: TcpStream,

    pub token: Token,

    interest: EventSet,

    send_queue: Vec<String>,


    debug: AtomicUsize
}

impl Client{
    pub fn new(event_loop: &mut EventLoop<Client>, address: &SocketAddr) -> Result<Client>{
        let stream = TcpStream::connect(address);
        if stream.is_ok(){
            let mut client = Client {
                socket: stream.unwrap(),
                send_queue: Vec::new(),
                token: CLIENT_TOKEN,
                interest: EventSet::readable() | EventSet::writable(),
                debug: AtomicUsize::new(0)
            };

            client.register(event_loop).ok();

            client.send_queue.push(String::from("Hell fucking yeah motherfucker"));
            client.send_queue.push(String::from("And so I wake in the morning"));
            client.send_queue.push(String::from("And I step outside"));
            client.send_queue.push(String::from("And I take a deep breath and I get real high"));
            client.send_queue.push(String::from("And I scream from the top of my lungs"));
            client.send_queue.push(String::from("What's going on?"));

            
            event_loop.run(&mut client).or_else(|e|{
                println!("Failed to start event loop!");
                Err(e)
            });

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
            PollOpt::edge() | PollOpt::oneshot()
        ).or_else(|e| {
            println!("Failed to register server! {:?}", e);
            Err(e)
        })
    }

    fn reregister(&mut self, event_loop: &mut EventLoop<Client>) -> Result<()>{
        event_loop.reregister(&self.socket, self.token, self.interest, PollOpt::edge())
            .or_else(|e|{
                println!("I am a sad panda, {:?}", e);
                Err(e)
        })
    }
}

impl Handler for Client{
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, _: &mut EventLoop<Client>) {
        println!("Begin client tick");

        println!("End client tick");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<Client>, token: Token, events: EventSet) {
        assert!(token != Token(0), "Token 0, y?????");

        if events.is_error(){
            println!("Client received error for token {:?}", token);
            return;
        }

        if events.is_hup(){
            println!("Oh shit, did the server crash or some shit?!");
            return;
        }

        if events.is_writable(){
            println!("TIME TO TALK MOTHERFUCKER");

            if !self.send_queue.is_empty(){
                if let Some(message) = self.send_queue.pop(){
                    match self.socket.try_write(message.as_bytes()){
                        Ok(Some(n)) => {
                            println!("Wrote {} bytes", n);
                        },
                        Ok(None) => {
                            println!("Nothing happened but it's okay I guess?");
                            self.send_queue.push(message);
                        },
                        Err(e) => {
                            println!("Oh fuck me god fucking damn it fucking shit fuck: {:?}", e);
                            self.send_queue.push(message);
                        }
                    };
                }
                else{
                    println!("Failed to pop message from queue!");
                }
                self.reregister(event_loop);
            }
            else{
                println!("WTF do you mean there's no messages for me?");
            }
        }

        if events.is_readable(){
            println!("OH shit, what've you got to say?");
        }

        self.debug.fetch_add(1, Ordering::SeqCst);
    }

    fn notify(&mut self, _: &mut EventLoop<Self>, _: Self::Message) {
        println!("Received notify!");
    }

    fn timeout(&mut self, _: &mut EventLoop<Self>, _: Self::Timeout) {
        println!("Received timeout");
    }
    fn interrupted(&mut self, _: &mut EventLoop<Self>) {
        println!("Interrupted! :O");
    }
}

#[test]
fn connect(){
    let addr = "127.0.0.1:6969".parse().unwrap();

    let mut event_loop = EventLoop::new().ok().expect("Failed to create event loop!");
    if let Ok(mut client) = Client::new(&mut event_loop, &addr){
        println!("Starting debug loop");

        loop{
            if client.debug.load(Ordering::Relaxed) > 0{
                break;
            }
        }

        println!("Done with this shit");
    }
    else{
        println!("Y u no client??");
    }
}

#[cfg(test)]
mod test {
    use mio::tcp::*;
    use mio::TryWrite;
    use std::net::SocketAddr;
    //use lag_client::Client;


}
