extern crate mio;
extern crate bytes;
extern crate byteorder;

mod authoritative;
mod client;

#[path="../shared/frame.rs"]
mod frame;
use frame::MessageHeader;

#[path="../shared/state.rs"]
mod state;
use state::ClientState;

use authoritative::AuthoritativeServer;

use std::net::SocketAddr;

fn main(){
    let address_str = "0.0.0.0:6969";
    let address: SocketAddr = address_str.parse::<SocketAddr>().expect("Failed to parse address!");

    println!("Starting server on address {}", address_str);
    let _ = AuthoritativeServer::new(address);

    println!("Done!");
}
