extern crate mio;

use mio::tcp::*;
use mio::TryWrite;
use mio::util::Slab;
use std::net::SocketAddr;
use std::io::Result;
use mio::{EventSet, EventLoop, Token, Handler, PollOpt};
use std::thread;
use std::thread::JoinHandle;
use std::sync::{Arc, RwLock};
//use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;

const CLIENT_TOKEN: mio::Token = mio::Token(1);


/// Contains data related to the client
pub struct ClientData{
    pub token: Token,

    interest: EventSet,

    send_queue: VecDeque<String>,
}

impl ClientData{
    fn new() -> ClientData{
        ClientData {
            send_queue: VecDeque::new(),
            token: CLIENT_TOKEN,
            interest: EventSet::readable() | EventSet::writable()
        }
    }
}


/// Maintains a reference to the client, the socket, and the thread join handle
struct ClientInterface{
    socket: TcpStream,

    thread_handle: Option<JoinHandle<()>>,

    client: Arc<RwLock<ClientData>>
}

impl ClientInterface{
    fn new(event_loop: Arc<RwLock<EventLoop<ClientInterface>>>, socket: TcpStream, client: Arc<RwLock<ClientData>>) -> Arc<RwLock<ClientInterface>>{
        let interface = Arc::new(RwLock::new(ClientInterface{
            socket: socket,
            thread_handle: None,
            client: client
        }));

        let thread_interface = interface.clone();
        println!("Before");
        let handle = thread::spawn(move||{
            println!("Hello from thread!");

            loop{
                if let Ok(mut event_loop) = event_loop.write(){
                    if let Ok(mut client_interface) = thread_interface.write(){
                        event_loop.run_once(&mut client_interface, None);
                    }
                }
            }
        });

        println!("after");

        if let Ok(mut interface_mut) = interface.write(){
            interface_mut.thread_handle = Some(handle);
        }

        return interface;
    }

    fn register(&mut self, event_loop: &mut EventLoop<ClientInterface>, client_ref: &Arc<RwLock<ClientData>>){
        if let Ok(client) = client_ref.read(){
            event_loop.register(
                &self.socket,
                client.token,
                client.interest,
                PollOpt::edge() | PollOpt::oneshot()
            ).or_else(|e| {
                println!("Failed to register server! {:?}", e);
                Err(e)
            });
        }
    }

    fn reregister(&mut self, event_loop: &mut EventLoop<ClientInterface>, client_ref: Arc<RwLock<ClientData>>){
        if let Ok(client) = client_ref.read(){
            event_loop.reregister(&self.socket, client.token, client.interest, PollOpt::edge())
                .or_else(|e|{
                    println!("I am a sad panda, {:?}", e);
                    Err(e)
            });
        }
    }
}

impl Handler for ClientInterface{
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, _: &mut EventLoop<ClientInterface>) {
        println!("Begin client tick");

        println!("End client tick");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<ClientInterface>, token: Token, events: EventSet) {
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

            if let Ok(mut client) = self.client.write(){
                if !client.send_queue.is_empty(){
                    if let Some(message) = client.send_queue.pop_front(){
                        match self.socket.try_write(message.as_bytes()){
                            Ok(Some(n)) => {
                                println!("Wrote {} bytes", n);
                            },
                            Ok(None) => {
                                println!("Nothing happened but it's okay I guess?");
                                client.send_queue.push_back(message);
                            },
                            Err(e) => {
                                println!("Oh fuck me god fucking damn it fucking shit fuck: {:?}", e);
                                client.send_queue.push_back(message);
                            }
                        };
                    }
                    else{
                        println!("Failed to pop message from queue!");
                    }
                }
                else{
                    println!("WTF do you mean there's no messages for me?");
                }
            }
        }

        if events.is_readable(){
            println!("OH shit, what've you got to say?");
        }

        let client_rereg = self.client.clone();
        self.reregister(event_loop, client_rereg);
        //self.debug.fetch_add(1, Ordering::SeqCst);
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


pub struct Client{
    data: Arc<RwLock<ClientData>>,
    interface: Arc<RwLock<ClientInterface>>,
    event_loop: Arc<RwLock<EventLoop<ClientInterface>>>
}

impl Client{
    /// Connect to the given socket and register with a threaded event loop
    pub fn connect(socket: TcpStream) -> Client{
        let event_loop = Arc::new(RwLock::new(EventLoop::new().ok().expect("Failed to create event loop!")));
        let client_data = Arc::new(RwLock::new(ClientData::new()));

        let interface_event_loop = event_loop.clone();
        let client_interface = ClientInterface::new(interface_event_loop, socket, client_data.clone());

        let mut client = Client{
            data: client_data,
            interface: client_interface,
            event_loop: event_loop
        };

        client.register();

        return client;
    }

    /// Register with the event loop
    fn register(&mut self){
        if let Ok(mut event_loop) = self.event_loop.write(){
            if let Ok(mut interface) = self.interface.write(){
                interface.register(&mut event_loop, &self.data);
            }
        }
    }
}

#[test]
fn connect(){
    let addr = "127.0.0.1:6969".parse().unwrap();

    let mut socket = TcpStream::connect(&addr);
    if socket.is_ok(){
        println!("Starting thing");

        let mut client = Client::connect(socket.unwrap());

        // if let Ok(mut client_ref) = client.write(){
        //     client_ref.send_queue.push_front(String::from("Hello, world!"));
        // }

        //loop{
            // if client.debug.load(Ordering::Relaxed) > 0{
            //     break;
            // }
        //}

        //println!("Done with this shit");
    }
    else{
        println!("Failed to open socket! {:?}", socket.unwrap_err());
    }
}

#[cfg(test)]
mod test {
    use mio::tcp::*;
    use mio::TryWrite;
    use std::net::SocketAddr;
    //use lag_client::Client;


}
