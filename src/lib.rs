extern crate mio;
extern crate crossbeam;

use mio::tcp::*;
use mio::TryWrite;
use mio::util::Slab;
use std::net::SocketAddr;
use std::io::Result;
use mio::{EventSet, EventLoop, Token, Handler, PollOpt};
use std::thread;
//use std::thread::{JoinHandle, JoinInner};
use std::sync::{Arc, RwLock, Condvar};
//use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;
//use crossbeam::Scope;

const CLIENT_TOKEN: mio::Token = mio::Token(1);

pub struct ClientNetworkInterface{
    socket: TcpStream,

    //thread_handle: Arc<Option<JoinHandle<()>>>,

    client: Arc<RwLock<Client>>
}

impl ClientNetworkInterface{
    pub fn new(event_loop: Arc<RwLock<EventLoop<ClientNetworkInterface>>>, socket: TcpStream, client: Arc<RwLock<Client>>) -> Arc<RwLock<ClientNetworkInterface>>{
        let interface = Arc::new(RwLock::new(ClientNetworkInterface{
            socket: socket,
            //thread_handle: Arc::new(None),
            client: client
        }));

        let thread_interface = interface.clone();
        println!("Before");
        thread::spawn(move||{
            println!("Hello from thread!");

            loop{
                if let Ok(mut event_loop) = event_loop.write(){
                    if let Ok(mut client_interface) = thread_interface.write(){
                        event_loop.run_once(&mut client_interface, None);
                    }
                }
            }
        });
        // crossbeam::scope(|scope|{
        //     scope.spawn(||{
        //         println!("thread start");
        //         loop{
        //             if let Ok(mut client_interface) = thread_interface.write(){
        //                 event_loop.run_once(&mut client_interface, None);
        //             }
        //         }
        //     });
        // });
        println!("after");

        // if let Ok(interface_mut) = interface.write(){
        //     interface_mut.thread_handle = handle;
        // }

        return interface;
    }

    fn register(&mut self, event_loop: &mut EventLoop<ClientNetworkInterface>, client_ref: &Arc<RwLock<Client>>){
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

    fn reregister(&mut self, event_loop: &mut EventLoop<ClientNetworkInterface>, client_ref: Arc<RwLock<Client>>){
        if let Ok(client) = client_ref.read(){
            event_loop.reregister(&self.socket, client.token, client.interest, PollOpt::edge())
                .or_else(|e|{
                    println!("I am a sad panda, {:?}", e);
                    Err(e)
            });
        }
    }
}



pub struct Client{
    pub token: Token,

    interest: EventSet,

    send_queue: VecDeque<String>,
}

impl Client{
    pub fn new() -> Client{
        Client {
            send_queue: VecDeque::new(),
            token: CLIENT_TOKEN,
            interest: EventSet::readable() | EventSet::writable()
        }
        // client.register(event_loop).ok();
        //
        // client.send_queue.push_back(String::from("Hell fucking yeah motherfucker"));
        // client.send_queue.push_back(String::from("And so I wake in the morning"));
        // client.send_queue.push_back(String::from("And I step outside"));
        // client.send_queue.push_back(String::from("And I take a deep breath and I get real high"));
        // client.send_queue.push_back(String::from("And I scream from the top of my lungs"));
        // client.send_queue.push_back(String::from("What's going on?"));
        //
        //
        // event_loop.run(&mut client).or_else(|e|{
        //     println!("Failed to start event loop!");
        //     Err(e)
        // });
    }
}

type RwArcClientInterface = Arc<RwLock<ClientNetworkInterface>>;

impl Handler for ClientNetworkInterface{
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, _: &mut EventLoop<ClientNetworkInterface>) {
        println!("Begin client tick");

        println!("End client tick");
    }

    fn ready(&mut self, event_loop: &mut EventLoop<ClientNetworkInterface>, token: Token, events: EventSet) {
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

        let mut client_rereg = self.client.clone();
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

#[test]
fn connect(){
    let addr = "127.0.0.1:6969".parse().unwrap();

    let mut socket = TcpStream::connect(&addr);
    if socket.is_ok(){

        println!("Starting thing");
        let mut event_loop = Arc::new(RwLock::new(EventLoop::new().ok().expect("Failed to create event loop!")));
        let mut client = Arc::new(RwLock::new(Client::new()));

        let interface_event_loop = event_loop.clone();
        let mut client_interface = ClientNetworkInterface::new(interface_event_loop, socket.unwrap(), client.clone());
        println!("Starting debug loop");

        if let Ok(mut event_loop_ref) = event_loop.write(){
            if let Ok(mut interface) = client_interface.write(){
                interface.register(&mut event_loop_ref, &client);
            }
        }

        if let Ok(mut client_ref) = client.write(){
            client_ref.send_queue.push_front(String::from("Hello, world!"));
        }


        loop{
            // if client.debug.load(Ordering::Relaxed) > 0{
            //     break;
            // }
        }

        println!("Done with this shit");
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
