use crate::api::{
    AnswerCustomMessage, AnswerError, AnswerGameSaved, AnswerNewObject, AnswerObjectCreated,
    AnswerPrint, AnswerReload, AnswerReturn, JsonMessage,
};
use anyhow::{anyhow, bail, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Display;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub struct ExternalEditorApi {
    listener: TcpListener,
}

impl ExternalEditorApi {
    pub fn new() -> Result<Self> {
        let server = TcpListener::bind("127.0.0.1:39998")?;
        Ok(Self { listener: server })
    }

    /// Sends a message to Tabletop Simulator
    pub fn send<T>(&mut self, message: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_message = serde_json::to_string(&message)?;
        let mut stream = TcpStream::connect("127.0.0.1:39999")?;
        stream.write_all(json_message.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    /// Waits for an answer with the correct id. Returns an Error if `AnswerError` is revieved.
    pub fn read<T>(&mut self) -> Result<T>
    where
        T: JsonMessage + DeserializeOwned,
    {
        let answer: Value = loop {
            let (mut stream, _addr) = self.listener.accept()?;
            let mut buffer = String::new();
            stream.read_to_string(&mut buffer).unwrap();

            let message: Value = serde_json::from_str(&buffer)?;
            let message_id = get_message_id(&message);

            match message_id {
                _ if message_id == T::MESSAGE_ID => break Ok(message),
                AnswerError::MESSAGE_ID => break Err(anyhow!(message)),
                _ => continue,
            };
        }?;
        Ok(serde_json::from_value(answer)?)
    }
}

pub fn read() -> Result<Box<dyn Display>> {
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
        _ => bail!("Can't find id"),
    }
}

fn helper<T>(answer: Value) -> Result<Box<dyn Display>>
where
    T: Display + DeserializeOwned + 'static,
{
    Ok(Box::new(serde_json::from_value::<T>(answer).unwrap()))
}

fn get_message_id(message: &Value) -> u8 {
    message["messageID"].as_u64().unwrap() as u8
}
