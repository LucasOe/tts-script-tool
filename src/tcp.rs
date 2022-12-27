use crate::api::{Answer, AnswerError, Message};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Sends a message to Tabletop Simulator and returns the answer as a Value::Object.
pub fn send<T: Message>(message: &T) -> Result<()> {
    let msg = serde_json::to_string(&message)?;
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();

    Ok(())
}

/// Waits for an answer with the correct id. Returns an Error if `AnswerError` is revieved.
pub fn read<T: Answer>() -> Result<T> {
    let listener = TcpListener::bind("127.0.0.1:39998")?;
    let answer: Value = loop {
        let (mut stream, _addr) = listener.accept()?;
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();

        let message: Value = serde_json::from_str(&buffer)?;
        let message_id = get_message_id(&message);

        match message_id {
            _ if message_id == T::MESSAGE_ID => break Ok(message),
            AnswerError::MESSAGE_ID => break Err(anyhow!(message)),
            _ => {}
        };
    }?;

    Ok(serde_json::from_value(answer)?)
}

fn get_message_id(message: &Value) -> u8 {
    message["messageID"].as_u64().unwrap() as u8
}
