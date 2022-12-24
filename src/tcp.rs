use crate::api::HasId;
use anyhow::{bail, Result};
use serde::{de, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Sends a message to Tabletop Simulator and returns the answer as a Value::Object.
pub fn send<T: de::DeserializeOwned + HasId, U: Serialize>(message: &U) -> Result<T> {
    let msg = serde_json::to_string(message)?;
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();
    // Wait for answer message with correct id and return it
    let message = loop {
        let message = read()?;
        let message_id = message["messageID"].as_u64().unwrap() as u8;
        if message_id == 3 {
            bail!(message);
        };
        if message_id == T::MESSAGE_ID {
            break message;
        };
    };

    Ok(serde_json::from_value(message)?)
}

/// Listen for message
// TODO: Add timeout when tabletop simulator doesn't accept listener
fn read() -> Result<Value> {
    let listener = TcpListener::bind("127.0.0.1:39998")?;
    let (mut stream, _addr) = listener.accept()?;
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    // Convert String into Value::Object
    let message: Value = serde_json::from_str(&buffer)?;
    Ok(message)
}
