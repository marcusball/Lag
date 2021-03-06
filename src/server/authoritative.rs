extern crate mio;
extern crate log;

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
use std::collections::HashMap;

#[path="../shared/frame.rs"]
mod frame;
use frame::{Message};

#[path="../shared/state.rs"]
mod state;
use state::{ClientState, GameState};


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
    message_queue: HashMap<Destination, Vec<Message>>,
    game_state: GameState,

    game_state_updated: bool
}

impl AuthoritativeServerState{
    pub fn new() -> AuthoritativeServerState{
        AuthoritativeServerState{
            token_counter: Arc::new(AtomicUsize::new(1)),
            // Max 128 connections
            clients: Arc::new(RwLock::new(Slab::new_starting_at(Token(2), 128))),
            message_queue: HashMap::new(),
            game_state: GameState::new(),
            game_state_updated: false
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

        info!("Starting authoritative server");
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

        info!("Running event loop...");

        loop{
            let timeout = event_loop.timeout_ms(123, 300).unwrap();
            event_loop.run_once(&mut server, None);
            let _ = event_loop.clear_timeout(timeout);
        }


        return server;
    }

    fn start_accept_loop(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>){
        info!("Beginning server accept loop");

        loop{
            let socket = match self.socket.accept(){
                Ok(s) => {
                    match s{
                        Some((socket,_)) => socket,
                        None => {
                            info!("Accept loop encountered WouldBlock");
                            return;
                        }
                    }
                },
                Err(e) => {
                    info!("Failed to accept new socket. Error: {:?}", e);
                    return;
                }
            };

            // If a client is successfully registered, this will be set to that client's token.
            let mut registered_token : Option<Token> = None;

            if let Ok(ref mut clients) = self.state.clients.write(){
                match &clients.insert_with(|token| {
                    info!("Inserting new connection from {:?}", token);
                    GameClient::new(socket, token)
                }) {
                    &Some(token) => {
                        info!("Insertion successful!");
                        let ref mut client: GameClient = clients[token];
                        match client.register(event_loop){
                            Ok(_) => {
                                registered_token = Some(token);
                            },
                            Err(e) => {
                                info!("Failed to register connection {:?} with event loop, error: {:?}", token, e);
                                //clients.remove(token);
                            }
                        }
                    },
                    &None => {
                        info!("Failed to insert!");
                    }
                }
            };

            if let Some(token) = registered_token{
                self.on_new_client_registered(token);
            }
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

    /// Called when a new client connects and has been registered with the event loop
    fn on_new_client_registered(&mut self, token: Token){
        info!("Registration successful!");
        self.construct_state_for_new_client(token);
    }

    /// Return TRUE if there are messages bound toward a client given by @token
    fn has_messages_for_client(&self, client: &GameClient) -> bool{
        // If there are any messages in messages bound for all clients, then we're good.
        !client.send_queue.is_empty()
    }

    /// Update the Game State with the given Client State
    fn update_client_in_game_state(&mut self, client_state: &ClientState){
        self.state.game_state.clients.insert(client_state.id, *client_state);
        self.state.game_state_updated = true;
    }

    fn construct_state_for_new_client(&mut self, token: Token){
        let state = ClientState::new(token.as_usize() as u32);
        self.update_client_in_game_state(&state);
        self.send_message_to_client(token, Message::new_client_update_message(&state));
    }

    fn send_message_to_client(&mut self, token: Token, message: Message){
        let destination = Destination::Client(token.clone());

        if self.state.message_queue.contains_key(&destination){
            if let Some(mailbox) = self.state.message_queue.get_mut(&destination){
                mailbox.push(message);
            }
        }
        else{
            self.state.message_queue.insert(destination, vec![message]);
        }
    }

    fn broadcast(&mut self, message: Message){
        let mut message_queue = &mut self.state.message_queue;
        if message_queue.contains_key(&Destination::Broadcast){
            let broadcast_queue = message_queue.get_mut(&Destination::Broadcast);
            if let Some(broadcast_queue) = broadcast_queue{
                broadcast_queue.push(message);
            }
            else{
                info!("Error: Failed to get mutable destination vec!");
            }
        }
        else{
            message_queue.insert(Destination::Broadcast, vec![message]);
        }
    }
}

impl Handler for AuthoritativeServer{
    type Timeout = u32;
    type Message = ();

    fn tick(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>) {
        //info!("Begin server tick!");

        if self.state.game_state_updated{
            let game_state_message = Message::GameStateUpdate(self.state.game_state.clients.values().map(|client| *client).collect::<Vec<ClientState>>());
            self.broadcast(game_state_message);
            self.state.game_state_updated = false;
        }

        if let Ok(mut clients) = self.state.clients.write(){
            for client in clients.iter_mut(){
                // Add any messages to the client which are destined specifically to this client.
                if let Some(mailbox) = self.state.message_queue.get_mut(&Destination::Client(client.token.clone())){
                    while let Some(message) = mailbox.pop(){
                        info!("Added message {:?}", message);
                        client.send_queue.push_back(message);
                    }
                }

                // Add any 'Broadcast' messages that exist to the client's send queue.
                if let Some(mailbox) = self.state.message_queue.get(&Destination::Broadcast){
                    for broadcast_message in mailbox{
                        info!("Added message {:?}", broadcast_message);
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

        //info!("End server tick!");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<AuthoritativeServer>, token: Token, events: EventSet) {
        assert!(token != Token(0), "We're not supposed to get a Token(0)!");

        info!("Ready for {:?}", token);

        if events.is_error(){
            info!("Error event for token {:?}", token);
            //Reset token?
            return;
        }

        if events.is_hup(){
            info!("OH FUCK NO, {:?} DID NOT JUST FUCKING HANG UP ON ME!", token);
            info!("I'M GOING TO FUCKING MURDER YOU FUCKER");

            if let Ok(mut clients) = self.state.clients.write(){
                &clients.remove(token);
                let _ = self.state.game_state.clients.remove(&(token.as_usize() as u32));
                self.state.game_state_updated = true;
            }
            //Reset?
            return;
        }

        if events.is_readable(){
            info!("GOT SHIT TO READ FROM MY BRAH {:?} HELLLL YEAH", token);

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
                                info!("--> Received text message");
                                let mut message_queue = &mut self.state.message_queue;
                                if message_queue.contains_key(&Destination::Broadcast){
                                    let broadcast_queue = message_queue.get_mut(&Destination::Broadcast);
                                    if let Some(broadcast_queue) = broadcast_queue{
                                        broadcast_queue.push(message);
                                    }
                                    else{
                                        info!("Error: Failed to get mutable destination vec!");
                                    }
                                }
                                else{
                                    message_queue.insert(Destination::Broadcast, vec![message]);
                                }
                            },

                            Message::Ping => {
                                //self.state.message_queue.insert(Destination::Broadcast, message);
                            },

                            Message::ClientUpdate(client_state) => {
                                info!("Received client update: {:?}", client_state);
                                if client_state.id as usize != token.as_usize(){
                                    info!("Error: Imposter trying to send client update! Claimed ID: {}, Token ID: {}", client_state.id, token.as_usize());
                                }
                                else{
                                    info!("Received client update packet! {:?}", client_state);
                                    self.update_client_in_game_state(&client_state);
                                }
                            },
                            Message::GameStateUpdate(_) => {
                                info!("Error: Received game state update from a client! ");
                            }
                        };
                    }
                    else{
                        info!("Error reading from client!");
                    }
            }
        }

        if events.is_writable(){
            info!("Oh shit, motherfucking {:?} is writable! Look at this guy!", token);

            //fucking write some shit
            self.get_client_mut(token, |client|{
                client.write()
            }).ok();
        }
    }
}
