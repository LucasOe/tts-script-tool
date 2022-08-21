use serde_json::json;
use std::io::Write;
use std::net::TcpStream;
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    let mut stream = TcpStream::connect("127.0.0.1:39999").expect("Couldn't connect to server.");
    println!("Connected!");

    let contents = fs::read_to_string(path).expect("Can't read file.");
    let msg = json!({
        "messageID": 3,
        "guid": "-1",
        "script": format!("{}", contents)
    })
    .to_string();
    stream.write(msg.as_bytes()).unwrap();
}
