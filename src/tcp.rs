use crate::api::{Answer, AnswerError};
use crate::{
    AnswerCustomMessage, AnswerGameSaved, AnswerNewObject, AnswerObjectCreated, AnswerPrint,
    AnswerReload, AnswerReturn, HasId,
};
use anyhow::{anyhow, bail, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Sends a message to Tabletop Simulator and returns the answer as a Value::Object.
pub fn send<T: Serialize>(message: &T) -> Result<()> {
    let msg = serde_json::to_string(&message)?;
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();

    Ok(())
}

/// Waits for an answer with the correct id. Returns an Error if `AnswerError` is revieved.
pub fn read<T: HasId + DeserializeOwned>() -> Result<T> {
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

#[rustfmt::skip]
pub fn read_any() -> Result<Box<dyn Answer>> {
    let listener = TcpListener::bind("127.0.0.1:39998")?;
    let (mut stream, _addr) = listener.accept()?;
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();

    let message: Value = serde_json::from_str(&buffer)?;
    let message_id = get_message_id(&message);

    match message_id {
        AnswerNewObject::MESSAGE_ID => helper::<AnswerNewObject>(message),
        AnswerReload::MESSAGE_ID => helper::<AnswerReload>(message),
        AnswerPrint::MESSAGE_ID => helper::<AnswerPrint>(message),
        AnswerError::MESSAGE_ID => helper::<AnswerError>(message),
        AnswerCustomMessage::MESSAGE_ID => helper::<AnswerCustomMessage>(message),
        AnswerReturn::MESSAGE_ID => helper::<AnswerReturn>(message),
        AnswerGameSaved::MESSAGE_ID => helper::<AnswerGameSaved>(message),
        AnswerObjectCreated::MESSAGE_ID => helper::<AnswerObjectCreated>(message),
        _ => bail!("Can't find id")
    }
}

fn helper<T: Answer + DeserializeOwned + 'static>(answer: Value) -> Result<Box<dyn Answer>> {
    Ok(Box::new(serde_json::from_value::<T>(answer).unwrap()))
}

fn get_message_id(message: &Value) -> u8 {
    message["messageID"].as_u64().unwrap() as u8
}
