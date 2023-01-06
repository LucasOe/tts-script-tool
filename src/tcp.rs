use crate::api::{
    AnswerCustomMessage, AnswerError, AnswerGameSaved, AnswerNewObject, AnswerObjectCreated,
    AnswerPrint, AnswerReload, AnswerReturn, MessageId,
};
use crate::{JsonMessage, MessageCustomMessage, MessageExectute, MessageGetScripts, MessageReload};
use anyhow::{bail, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub struct ExternalEditorApi {}

impl ExternalEditorApi {
    pub fn new() -> Self {
        Self {}
    }

    fn send<T>(&self, message: T)
    where
        T: Serialize,
    {
        let json_message = serde_json::to_string(&message).unwrap();
        let mut stream = TcpStream::connect("127.0.0.1:39999").unwrap();
        stream.write_all(json_message.as_bytes()).unwrap();
        stream.flush().unwrap();
    }

    pub fn read(&self) -> Result<Box<dyn JsonMessage>> {
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

    /// Waits for an answer with the correct id and returns it
    pub fn wait<T>(&self) -> T
    where
        T: MessageId + DeserializeOwned + Clone + 'static,
    {
        let answer = loop {
            let answer = self.read().unwrap();
            let message_id = answer.message_id();
            match message_id {
                _ if message_id == T::MESSAGE_ID => break answer,
                _ => {}
            }
        };
        let downcast = answer.as_any().downcast_ref::<T>().unwrap();
        downcast.clone()
    }

    pub fn get_scripts(&self) -> AnswerReload {
        self.send(MessageGetScripts::new());
        self.wait()
    }

    pub fn reload(&self, script_states: Value) -> AnswerReload {
        self.send(MessageReload::new(script_states));
        self.wait()
    }

    pub fn custom_message(&self, message: Value) {
        self.send(MessageCustomMessage::new(message));
    }

    pub fn execute(&self, script: String) -> AnswerReturn {
        self.send(MessageExectute::new(script));
        self.wait()
    }
}

fn helper<T>(answer: Value) -> Result<Box<dyn JsonMessage>>
where
    T: JsonMessage + DeserializeOwned + 'static,
{
    Ok(Box::new(serde_json::from_value::<T>(answer).unwrap()))
}

fn get_message_id(message: &Value) -> u8 {
    message["messageID"].as_u64().unwrap() as u8
}
