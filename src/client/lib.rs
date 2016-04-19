extern crate mio;
extern crate byteorder;

#[path="../shared/frame.rs"]
pub mod frame;
use frame::{MessageFrame, ToFrame, Message};

#[path="../shared/state.rs"]
pub mod state;
use state::{ClientState, Position, Rotation, Transform};

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
use std::time::Duration;

const CLIENT_TOKEN: mio::Token = mio::Token(1);

/// Estimation of the average number of messages that will be received per tick.
/// Used as the capacity value in Vec::with_capacity(capacity: usize);
const RECEIVED_MESSAGES_PER_TICK: usize = 2;


/// Contains data related to the client
pub struct ClientData{
    pub id: u32,

    pub token: Token,

    interest: EventSet,

    send_queue: VecDeque<MessageFrame>,

    // Buffer of received messages
    receive_queue: Vec<Message>,

    client_state: ClientState,

    state_updated: bool
}

impl ClientData{
    fn new() -> ClientData{
        ClientData {
            id: 0,
            send_queue: VecDeque::new(),
            token: CLIENT_TOKEN,
            interest: EventSet::readable(),
            receive_queue: Vec::with_capacity(RECEIVED_MESSAGES_PER_TICK),
            client_state: ClientState::new(CLIENT_TOKEN.as_usize() as u32),
            state_updated: false
        }
    }

    fn set_writable(&mut self){
        self.interest.insert(EventSet::writable());
    }

    fn set_read_only(&mut self){
        self.interest.remove(EventSet::writable());
        self.interest.insert(EventSet::readable());
    }

    fn has_messages_to_send(&self) -> bool{
        !self.send_queue.is_empty()
    }
}


/// Maintains a reference to the client, the socket, and the thread join handle
struct ClientInterface{
    socket: TcpStream,

    thread_handle: Option<JoinHandle<()>>,

    client: Arc<RwLock<ClientData>>,

    is_connected: bool,
}

impl ClientInterface{
    fn new(event_loop: Arc<RwLock<EventLoop<ClientInterface>>>, socket: TcpStream, client: Arc<RwLock<ClientData>>) -> Arc<RwLock<ClientInterface>>{
        let interface = Arc::new(RwLock::new(ClientInterface{
            socket: socket,
            thread_handle: None,
            client: client,
            is_connected: true
        }));

        let thread_interface = interface.clone();
        let handle = thread::spawn(move||{
            loop{

                if let Ok(mut client_interface) = thread_interface.write(){
                    if let Ok(mut event_loop) = event_loop.write(){
                        let timeout = event_loop.timeout_ms(123, 300).unwrap();
                        event_loop.run_once(&mut client_interface, None);
                        let _ = event_loop.clear_timeout(timeout);
                    }
                }

                if let Ok(client_interface) = thread_interface.try_read(){
                    if !client_interface.is_connected{
                        println!("The client has disconnected from the server!");
                        break;
                    }
                }

                thread::sleep(Duration::new(0,100));
            }
        });

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

        println!("Begin client read message");

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
                    },
                    Message::ClientUpdate(_) =>{
                        println!("Received client update packet!");
                    },
                    Message::GameStateUpdate( _ ) => {
                        println!("Received game state update!");
                    }
                }
                return Ok(message);
            },
            Err(e) => {
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
        if let Ok(mut client) = self.client.try_write(){
            client.set_read_only();
        }
    }

    fn has_messages_to_send(&self) -> bool{
        if let Ok(client) = self.client.try_read(){
            return client.has_messages_to_send();
        }
        return false;
    }

    fn set_socket_disconnected(&mut self){
        self.is_connected = false;
    }
}

impl Handler for ClientInterface{
    type Timeout = u32;
    type Message = ();

    fn tick(&mut self, event_loop: &mut EventLoop<ClientInterface>) {
        //println!("Begin client tick");

        if let Ok(mut data) = self.client.try_write(){
            if data.state_updated{
                // @TODO: Check if there's already a ClientState message in the output queue
                let client_state = data.client_state;
                data.send_queue.push_back(Message::new_client_update_message(&client_state).to_frame());
                data.state_updated = false;
            }
        }


        match self.has_messages_to_send(){
            true  => { self.set_writable(); },
            false => { self.set_read_only(); }
        }

        if self.is_connected{
            self.reregister(event_loop);
        }

        //println!("End client tick");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<ClientInterface>, token: Token, events: EventSet) {
        assert!(token != Token(0), "Token 0, y?????");

        if events.is_error(){
            println!("Client received error for token {:?}", token);
            return;
        }

        if events.is_hup(){
            println!("Oh shit, did the server ({:?}) crash or some shit?!", token);
            return;
        }

        if events.is_writable(){
            println!("TIME TO TALK MOTHERFUCKER");

            if let Ok(mut client) = self.client.try_write(){
                if !client.send_queue.is_empty(){
                //     let output_buffer = client.send_queue.iter()
                //                     .map(|mes| mes.to_bytes() ).
                //                     fold(Vec::new(), |mut buf, mut mes|{ buf.append(&mut mes); buf });
                   //
                   //
                //     match self.socket.try_write(output_buffer.as_slice()){
                //        Ok(Some(n)) => {
                //            println!("Wrote {} bytes", n);
                //            client.send_queue.clear();
                //        },
                //        Ok(None) => {
                //            println!("Nothing happened but it's okay I guess?");
                //            //client.send_queue.push_back(message_frame);
                //        },
                //        Err(e) => {
                //            println!("Oh fuck me god fucking damn it fucking shit fuck: {:?}", e);
                //            //client.send_queue.push_back(message_frame);
                //        }
                //    };
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
            }
            else{
                println!("Nothing to write...");
            }
        }

        if events.is_readable(){
            println!("OH shit, what've you got to say?");

            let received_message = self.read();


            match received_message{
                Ok(message) => {
                    if let Ok(mut data) = self.client.write(){
                        if let Message::ClientUpdate(client_state) = message{
                            println!("Received client ID: {}", client_state.id);
                            data.client_state.id = client_state.id;
                            data.id = client_state.id;
                        }
                        else{
                            data.receive_queue.push(message);
                        }
                    }
                },
                Err(e) => {
                    println!("Error trying to read! {:?}", e);
                    if let Some(error_number) = e.raw_os_error(){
                        if error_number == 10057{
                            println!("Socket is not connected!");
                            self.set_socket_disconnected();
                        }
                    }
                }
            }
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
        let socket = TcpStream::connect(address);
        match socket{
            Ok(socket) => {
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
            },
            Err(e) => {
                println!("Failed to connect! {:?}", e);
                return Err(e);
            }
        }
    }

    /// Register with the event loop
    fn register(&mut self){
        if let Ok(mut interface) = self.interface.write(){
            if let Ok(mut event_loop) = self.event_loop.write(){

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

    pub fn read(&mut self) -> Result<Message>{
        if let Ok(mut interface) = self.interface.write(){
            // Suddenly realizing that I just wrote some really confusing code here.
            return interface.read();
        }
        else{
            return Err(Error::new(ErrorKind::Other, String::from("Failed to read from client interface!")));
        }
    }

    pub fn is_connected(&self) -> bool{
        if let Ok(interface) = self.interface.try_read(){
            return interface.is_connected;
        }
        // Only return disconnected when we know for sure it's true
        return true;
    }

    pub fn pop_received_messages(&mut self) -> Option<Vec<Message>>{
        if let Ok(mut data) = self.data.write(){
            if !data.receive_queue.is_empty(){
                return Some(std::mem::replace(&mut data.receive_queue, Vec::with_capacity(RECEIVED_MESSAGES_PER_TICK)));
            }
            else{
                return None;
            }
        }
        return None;
    }

    fn get_client_state(&self) -> Result<ClientState>{
        if let Ok(data) = self.data.read(){
            return Ok(data.client_state);
        }
        return Err(Error::new(ErrorKind::Other, String::from("Failed to read client state!")));
    }

    /// Update the @position and @rotation of the client
    pub fn set_transform(&mut self, transform: Transform){
        if let Ok(mut data) = self.data.try_write(){
            data.client_state.position = transform.position;
            data.client_state.rotation = transform.rotation;
            data.state_updated = true;
        }

        // if let Ok(mut data) = self.data.try_write(){
        //     data.set_writable();
        // }
        // else{ println!("Failed to write to client data"); }
        //
        // if let Ok(mut event_loop) = self.event_loop.try_write(){
        //     if let Ok(mut interface) = self.interface.try_write(){
        //         interface.reregister(&mut event_loop);
        //     }
        //     else{ println!("Failed to write to client interface"); }
        // }
        // else{ println!("Failed to write to event loop"); }
    }

    /// Update the @position of the client
    /// Maintains the current rotation
    pub fn set_position(&mut self, position: Position){
        let current_rotation = self.get_rotation();
        self.set_transform(Transform::from_components(position, current_rotation));
    }

    /// Update the @rotation of the client
    /// Maintains the current position
    pub fn set_rotation(&mut self, rotation: Rotation){
        let current_position = self.get_position();
        self.set_transform(Transform::from_components(current_position, rotation));
    }

    /// Get the client's current position
    pub fn get_position(&self) -> Position{ self.get_transform().position }

    /// Get the client's current rotaton
    pub fn get_rotation(&self) -> Rotation{ self.get_transform().rotation }

    /// Get the client's position and rotation
    pub fn get_transform(&self) -> Transform{
        if let Ok(data) = self.data.read(){
            return Transform::from_components(data.client_state.position, data.client_state.rotation);
        }
        panic!();
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

            for _ in 1..5{
                thread::sleep(Duration::new(2,0));
            }
        }
    }
}
