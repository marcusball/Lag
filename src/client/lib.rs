extern crate mio;
extern crate byteorder;

#[path="../shared/frame.rs"]
mod frame;
use frame::{MessageFrame, ToFrame, Message};

use mio::tcp::*;
use mio::TryWrite;
use mio::util::Slab;
use std::net::SocketAddr;
use std::io::{Result, Read, Error, ErrorKind};
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

    send_queue: VecDeque<MessageFrame>,
}

impl ClientData{
    fn new() -> ClientData{
        ClientData {
            send_queue: VecDeque::new(),
            token: CLIENT_TOKEN,
            interest: EventSet::readable()
        }
    }

    fn set_writable(&mut self){
        self.interest.insert(EventSet::writable());
    }

    fn set_read_only(&mut self){
        self.interest.remove(EventSet::writable());
    }

    fn has_messages_to_send(&self) -> bool{
        !self.send_queue.is_empty()
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

    fn register(&mut self, event_loop: &mut EventLoop<ClientInterface>){
        if let Ok(client) = self.client.read(){
            event_loop.register(
                &self.socket,
                client.token,
                client.interest,
                PollOpt::edge() | PollOpt::oneshot()
            ).or_else(|e| {
                println!("Failed to register server! {:?}", e);
                Err(e)
            }).ok();
        }
    }

    fn reregister(&mut self, event_loop: &mut EventLoop<ClientInterface>){
        if let Ok(client) = self.client.read(){
            event_loop.reregister(&self.socket, client.token, client.interest, PollOpt::edge())
                .or_else(|e|{
                    println!("I am a sad panda, {:?}", e);
                    Err(e)
            }).ok();
        }
    }

    pub fn read(&mut self) -> Result<Message>{
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

    fn set_writable(&mut self){
        if let Ok(mut client) = self.client.write(){
            client.set_writable();
        }
    }

    fn set_read_only(&mut self){
        if let Ok(mut client) = self.client.write(){
            client.set_read_only();
        }
    }

    fn has_messages_to_send(&self) -> bool{
        if let Ok(client) = self.client.read(){
            return client.has_messages_to_send();
        }
        return false;
    }
}

impl Handler for ClientInterface{
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, event_loop: &mut EventLoop<ClientInterface>) {
        println!("Begin client tick");

        if self.has_messages_to_send(){
            self.set_writable();
        }
        else{
            self.set_read_only();
        }

        self.reregister(event_loop);

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
                    if let Some(message_frame) = client.send_queue.pop_front(){
                        match self.socket.try_write(message_frame.to_bytes().as_slice()){
                            Ok(Some(n)) => {
                                println!("Wrote {} bytes", n);
                            },
                            Ok(None) => {
                                println!("Nothing happened but it's okay I guess?");
                                client.send_queue.push_back(message_frame);
                            },
                            Err(e) => {
                                println!("Oh fuck me god fucking damn it fucking shit fuck: {:?}", e);
                                client.send_queue.push_back(message_frame);
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
            else{
                println!("Nothing to write...");
            }
        }

        if events.is_readable(){
            println!("OH shit, what've you got to say?");

            let _ = self.read();
        }

        //let client_rereg = self.client.clone();
        //self.reregister(event_loop, &client_rereg);
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
    pub fn connect(address: &SocketAddr) -> Result<Client>{
        let socket = try!(TcpStream::connect(address));
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

            return Ok(client);
    }

    /// Register with the event loop
    fn register(&mut self){
        if let Ok(mut event_loop) = self.event_loop.write(){
            if let Ok(mut interface) = self.interface.write(){
                interface.register(&mut event_loop);
            }
        }
    }

    /// Register with the event loop
    fn reregister(&mut self){
        if let Ok(mut event_loop) = self.event_loop.write(){
            if let Ok(mut interface) = self.interface.write(){
                interface.reregister(&mut event_loop);
            }
        }
    }

    pub fn send_message<T: ToFrame>(&mut self, message: &T){
        if let Ok(mut data) = self.data.write(){
            data.send_queue.push_back(message.to_frame());
            data.set_writable();
        } else { return; }

        self.reregister();
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use std::net::SocketAddr;
    use std::thread;
    use super::Client;

    #[path="../../shared/frame.rs"]
    mod frame;
    use frame::{MessageHeader, ToFrame, Message};

    #[test]
    fn connect(){
        let addr = "127.0.0.1:6969".parse().unwrap();

        if let Ok(mut client) = Client::connect(&addr){
            let message = Message::new_text_message(String::from("Hello, world!"));
            client.send_message(&message);
            // if let Ok(mut client_ref) = client.write(){
            //     client_ref.send_queue.push_front(String::from("Hello, world!"));
            // }

            thread::sleep(Duration::new(1,0));

        }

            //loop{
                // if client.debug.load(Ordering::Relaxed) > 0{
                //     break;
                // }
            //}

            //loop {}

            //println!("Done with this shit");
    }
}
