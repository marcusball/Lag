extern crate mio;

use client::GameClient;

//use mio::{TryRead, TryWrite};
use mio::tcp::*;
use mio::util::Slab;
use mio::Token;
use mio::{EventLoop, EventSet, PollOpt, Handler};
//use bytes::{Buf, Take};
//use std::mem;
use std::net::SocketAddr;
//use std::io::Cursor;
//use std::thread;
//use std::sync::mpsc;
use std::sync::{Arc, RwLock};
//use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::atomic::AtomicUsize;
use std::collections::{HashMap, VecDeque};

#[path="../shared/frame.rs"]
mod frame;
use frame::{Message};


const SERVER_TOKEN: mio::Token = mio::Token(1);

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum Destination{
    Client(Token),
    Broadcast
}

#[derive(Clone)]
pub struct AuthoritativeServerState{
    clients: Arc<RwLock<Slab<GameClient>>>,
    token_counter: Arc<AtomicUsize>,
    message_queue: HashMap<Destination, Vec<Message>>
}

impl AuthoritativeServerState{
    pub fn new() -> AuthoritativeServerState{
        AuthoritativeServerState{
            token_counter: Arc::new(AtomicUsize::new(1)),
            // Max 128 connections
            clients: Arc::new(RwLock::new(Slab::new_starting_at(Token(2), 128))),
            message_queue: HashMap::new()
        }
    }
}

pub struct AuthoritativeServer{
    // The socket on which this server is listening
    socket: TcpListener,

    // The token assigned to this server's own connection
    token: Token,

    // Current state for the server
    state: AuthoritativeServerState
}

impl AuthoritativeServer{
    pub fn new(address: SocketAddr) -> AuthoritativeServer{
        let server_state = AuthoritativeServerState::new();
        let server_state_clone = server_state.clone();

        println!("Starting authoritative server");
        let server_socket = TcpListener::bind(&address).expect("Failed to start socket listener!");

        let mut server = AuthoritativeServer{
            socket: server_socket,
            token: SERVER_TOKEN,
            state: server_state_clone
        };

        let mut event_loop = EventLoop::new().expect("Failed to create server event loop!");

        event_loop.register(&server.socket,
                            SERVER_TOKEN,
                            EventSet::readable(),
                            PollOpt::edge()).expect("Failed to register server with event loop!");

        println!("Running event loop...");
        event_loop.run(&mut server).expect("Failed to run event loop!");

        return server;
    }

    fn start_accept_loop(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>){
        println!("Beginning server accept loop");

        loop{
            let socket = match self.socket.accept(){
                Ok(s) => {
                    match s{
                        Some((socket,_)) => socket,
                        None => {
                            println!("Accept loop encountered WouldBlock");
                            return;
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to accept new socket. Error: {:?}", e);
                    return;
                }
            };

            if let Ok(ref mut clients) = self.state.clients.write(){
                match &clients.insert_with(|token| {
                    println!("Inserting new connection from {:?}", token);
                    GameClient::new(socket, token)
                }) {
                    &Some(token) => {
                        println!("Insertion successful!");
                        let ref mut client: GameClient = clients[token];
                        match client.register(event_loop){
                            Ok(_) => { println!("Registration successful!"); },
                            Err(e) => {
                                println!("Failed to register connection {:?} with event loop, error: {:?}", token, e);
                                //clients.remove(token);
                            }
                        }
                    },
                    &None => {
                        println!("Failed to insert!");
                    }
                }
            };
        }
    }

    fn get_client_mut<'a, F, R>(&'a mut self, token: Token, mut action: F) -> Result<R, String>
        where F: FnMut(&mut GameClient) -> R {
        if let Ok(mut clients) = self.state.clients.write(){
            if clients.contains(token){
                let ref mut client = clients.get_mut(token).unwrap();

                let return_val = action(client);

                return Ok(return_val);
            }
        }
        Err(String::from(format!("No client exists with token {:?}", token)))
    }

    fn get_client<'a, F, R>(&'a self, token: Token, action: F) -> Result<R, String>
        where F: Fn(&GameClient) -> R{
        if let Ok(clients) = self.state.clients.read(){
            if clients.contains(token){
                let ref mut client = clients.get(token).expect("Clients contains token, but was unable to access client!");

                let return_val = action(client);

                return Ok(return_val);
            }
        }
        Err(String::from(format!("No client exists with token {:?}", token)))
    }

    /// Return TRUE if there are messages bound toward a client given by @token
    fn has_messages_for_client(&self, client: &GameClient) -> bool{
        // If there are any messages in messages bound for all clients, then we're good.
        !client.send_queue.is_empty()
    }
}

impl Handler for AuthoritativeServer{
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>) {
        println!("Begin server tick!");

        if let Ok(mut clients) = self.state.clients.write(){
            for client in clients.iter_mut(){
                // Add any messages to the client which are destined specifically to this client.
                if let Some(mailbox) = self.state.message_queue.get_mut(&Destination::Client(client.token.clone())){
                    while let Some(message) = mailbox.pop(){
                        println!("Added message {:?}", message);
                        client.send_queue.push_back(message);
                    }
                }

                // Add any 'Broadcast' messages that exist to the client's send queue.
                if let Some(mailbox) = self.state.message_queue.get(&Destination::Broadcast){
                    for broadcast_message in mailbox{
                        println!("Added message {:?}", broadcast_message);
                        client.send_queue.push_back(broadcast_message.clone());
                    }
                }

                // Reregister the client with the event loop,
                // so we continue to receive events for this client
                let client_register_writable = self.has_messages_for_client(client);
                client.reregister(event_loop, client_register_writable).ok();
            }
        }

        // Clear out the broadcast queue
        // Warning: This assumes no failures to send
        let ref mut message_queue = self.state.message_queue;
        if let Some(broadcast_queue) = message_queue.get_mut(&Destination::Broadcast){
            broadcast_queue.clear();
        }

        println!("End server tick!");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>, token: Token, events: EventSet) {
        assert!(token != Token(0), "We're not supposed to get a Token(0)!");

        println!("Ready for {:?}", token);

        if events.is_error(){
            println!("Error event for token {:?}", token);
            //Reset token?
            return;
        }

        if events.is_hup(){
            println!("OH FUCK NO, {:?} DID NOT JUST FUCKING HANG UP ON ME!", token);
            println!("I'M GOING TO FUCKING MURDER YOU FUCKER");

            if let Ok(mut clients) = self.state.clients.write(){
                &clients.remove(token);
            }
            //Reset?
            return;
        }

        if events.is_writable(){
            println!("Oh shit, motherfucking {:?} is writable! Look at this guy!", token);

            //fucking write some shit
            self.get_client_mut(token, |client|{
                client.write()
            }).ok();
        }

        if events.is_readable(){
            println!("GOT SHIT TO READ FROM MY BRAH {:?} HELLLL YEAH", token);

            if self.token == token{
                self.start_accept_loop(event_loop);
            }
            else{
                let message = self.get_client_mut(token, |client|{
                    return client.read();
                }).ok();

                if let Some(Ok(message)) = message{
                    match message{
                        Message::Text{ message: _} => {
                            let mut message_queue = &mut self.state.message_queue;
                            if message_queue.contains_key(&Destination::Broadcast){
                                let broadcast_queue = message_queue.get_mut(&Destination::Broadcast);
                                if let Some(broadcast_queue) = broadcast_queue{
                                    broadcast_queue.push(message);
                                }
                                else{
                                    println!("Error: Failed to get mutable destination vec!");
                                }
                            }
                            else{
                                message_queue.insert(Destination::Broadcast, vec![message]);
                            }
                        },

                        Message::Ping => {
                            //self.state.message_queue.insert(Destination::Broadcast, message);
                        }
                    };
                }
                else{
                    println!("Error reading from client!");
                }
            }
        }
    }
}
