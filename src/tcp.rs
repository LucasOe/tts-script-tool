use crate::api::Answer;
use anyhow::{bail, Result};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Sends a message to Tabletop Simulator and returns the answer as a Value::Object.
pub fn send(msg: String, id: u64) -> Result<Answer> {
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();
    // Wait for answer message with correct id and return it
    let (message, id) = loop {
        let message = read()?;
        let message_id = message["messageID"].as_u64().unwrap();
        if message_id == id {
            break (message, message_id);
        }
    };

    // Convert Value into Answer
    match id {
        1 => Ok(Answer::AnswerReload(serde_json::from_value(message)?)),
        5 => Ok(Answer::AnswerReturn(serde_json::from_value(message)?)),
        _ => bail!("Message couldn't be deserialized into an Answer"),
    }
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
