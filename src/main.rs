use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    // get save data
    let data = send(
        json!({
            "messageID": 0,
        })
        .to_string(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(&data).unwrap();
    println!("{:?}", result);

    // execute script
    let data = send(
        json!({
            "messageID": 3,
            "guid":"-1",
            "script":"print(\"Hello, Mars\")"
        })
        .to_string(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(&data).unwrap();
    println!("{:?}", result);
}

fn send(msg: String) -> Option<String> {
    let mut stream = TcpStream::connect("127.0.0.1:39999").unwrap();
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();

    let listener = TcpListener::bind("127.0.0.1:39998").unwrap();
    match listener.accept() {
        Ok((stream, _addr)) => Some(read(&stream)),
        Err(_) => None,
    }
}

fn read(mut stream: &TcpStream) -> String {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    buffer
}
