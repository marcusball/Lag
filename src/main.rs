extern crate mio;
use mio::*;
use mio::tcp::*;

extern crate http_muncher;
use http_muncher::{Parser, ParserHandler};

extern crate sha1;
extern crate rustc_serialize;

use rustc_serialize::base64::{ToBase64, STANDARD};

use std::net::SocketAddr;
use std::collections::HashMap;

use std::cell::RefCell;
use std::rc::Rc;

struct HttpParser{
    current_key: Option<String>,
    headers: Rc<RefCell<HashMap<String,String>>>
}

impl ParserHandler for HttpParser{
    fn on_header_field(&mut self, s: &[u8]) -> bool{
        self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, s: &[u8]) -> bool{
        self.headers.borrow_mut()
            .insert(self.current_key.clone().unwrap(),
                    std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_headers_complete(&mut self) -> bool{
        false
    }
}

fn gen_key(key: &String) -> String{
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    m.update(key.as_bytes());
    m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

    m.output(&mut buf);

    return buf.to_base64(STANDARD);
}

struct WebSocketClient {
    socket: TcpStream,
    http_parser: Parser<HttpParser>,

    //Headers declaration
    headers: Rc<RefCell<HashMap<String, String>>>,

    interest: EventSet
}

impl WebSocketClient{
    fn read(&mut self){
        loop{
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf){
                Err(e) =>{
                    println!("Error while reading socket: {:?}",e);
                    return
                },
                Ok(None) =>
                    //Socket buffer received no additional bytes
                    break,
                Ok(Some(len)) => {
                    self.http_parser.parse(&buf[0..len]);
                    if self.http_parser.is_upgrade(){
                        // ..
                        break;
                    }
                }
            }
        }
    }

    fn new(socket: TcpStream) -> WebSocketClient{
        let headers = Rc::new(RefCell::new(HashMap::new()));

        WebSocketClient{
            socket: socket,
            headers: headers.clone(),
            http_parser: Parser::request(HttpParser{
                current_key: None,
                headers: headers.clone()
            })
        }
    }
}

struct DemoSocketServer{
    socket: TcpListener,
    clients: HashMap<Token, WebSocketClient>,
    token_counter: usize
}

const SERVER_TOKEN: Token = Token(0);

impl Handler for DemoSocketServer{
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<DemoSocketServer>,
             token: Token, events: EventSet){
        match token{
            SERVER_TOKEN => {
                let client_socket = match self.socket.accept() {
                    Err(e) => {
                        println!("Accept error: {}", e);
                        return;
                    },
                    Ok(None) => unreachable!("Socket has returned 'None'"),
                    Ok(Some((sock, addr))) => sock
                };

                self.token_counter += 1;
                let new_token = Token(self.token_counter);

                self.clients.insert(new_token, WebSocketClient::new(client_socket));
                event_loop.register(&self.clients[&new_token].socket,
                                    new_token, EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot()).unwrap();
            },
            token => {
                let mut client = self.clients.get_mut(&token).unwrap();
                client.read();
                event_loop.reregister(&client.socket, token, client.interest,
                                      PollOpt::edge() | PollOpt::oneshot()).unwrap();
            }
        }
    }
}

fn main(){
    let address = "0.0.0.0:6969".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut event_loop = EventLoop::new().unwrap();

    //Create new instance of handler struct
    let mut server = DemoSocketServer{
        token_counter: 1,
        clients: HashMap::new(),
        socket: server_socket
    };

    event_loop.register(&server.socket,
        SERVER_TOKEN, EventSet::readable(),
        PollOpt::edge()).unwrap();

    //provite event loop with mutable reference
    event_loop.run(&mut server).unwrap();
}
