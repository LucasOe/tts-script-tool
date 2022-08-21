use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{env, fs};

#[allow(unused_variables)]
fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    let stream = TcpStream::connect("127.0.0.1:39999").expect("Couldn't connect to server.");
    println!("Connected!");

    // get objects
    let msg = json!({
        "messageID": 0
    })
    .to_string();
    write(&stream, msg);

    let data = listen().unwrap();
    let result: Value = serde_json::from_str(&data).unwrap();
    println!("{:?}", result);

    // execute script
    let new_stream = TcpStream::connect("127.0.0.1:39999").expect("Couldn't connect to server.");
    let msg = json!({
        "messageID": 3,
        "guid": "-1",
        "script": "print(\"Hello, World!\")"
    })
    .to_string();
    write(&new_stream, msg);

    let data = listen().unwrap();
    let result: Value = serde_json::from_str(&data).unwrap();
    println!("{:?}", result);

    let contents = fs::read_to_string(path).expect("Can't read file.");
}

fn listen() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:39998").unwrap();
    match listener.accept() {
        Ok((stream, _addr)) => Some(read(&stream)),
        Err(_) => None,
    }
}

fn write(mut stream: &TcpStream, msg: String) {
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn read(mut stream: &TcpStream) -> String {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    buffer
}
