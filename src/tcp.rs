use anyhow::Result;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Sends a message to Tabletop Simulator and returns the answer as a Value::Object.
pub fn send(msg: String, id: u64) -> Result<Value> {
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();
    // Wait for answer message and return it
    let message = loop {
        let message = read()?;
        let message_id = message["messageID"].as_u64().unwrap();
        if message_id == id {
            break message;
        }
    };
    Ok(message)
}

/// Listen for message
// TODO: Add timeout when tabletop simulator doesn't accept listener
fn read() -> Result<Value> {
    let listener = TcpListener::bind("127.0.0.1:39998")?;
    let (mut stream, _addr) = listener.accept()?;
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    // Convert String into Value::Object and return message
    let message: Value = serde_json::from_str(&buffer)?;
    Ok(message)
}