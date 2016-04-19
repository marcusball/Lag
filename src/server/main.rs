extern crate mio;
extern crate bytes;
extern crate byteorder;
#[macro_use]
extern crate log;
extern crate env_logger;

mod authoritative;
mod client;

#[path="../shared/frame.rs"]
mod frame;

#[path="../shared/state.rs"]
mod state;
use state::ClientState;

use authoritative::AuthoritativeServer;

use std::net::SocketAddr;

fn main(){
    env_logger::init().unwrap();

    
    let address_str = "0.0.0.0:6969";
    let address: SocketAddr = address_str.parse::<SocketAddr>().expect("Failed to parse address!");

    info!("Starting server on address {}", address_str);
    let _ = AuthoritativeServer::new(address);

    info!("Done!");
}
