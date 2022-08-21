use std::env;
use std::net::TcpStream;

#[allow(unused_variables)]
fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    connect();
}

#[allow(unused_variables)]
fn connect() {
    match TcpStream::connect("127.0.0.1:39999") {
        Ok(stream) => {
            println!("Connected!");
        }
        Err(e) => {
            println!("Can't connect: {}", e);
        }
    };
}
