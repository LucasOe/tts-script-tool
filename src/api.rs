//! Reference: https://api.tabletopsimulator.com/externaleditorapi/
//!
//! Communication between the editor and TTS occurs via two localhost TCP connections:
//! one where TTS listens for messages and one where ttsst listens for messages.
//! All communication messages are JSON.

use crate::tcp::send;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait HasId {
    const MESSAGE_ID: u8;
}

#[derive(Serialize, Debug)]
pub struct MessageGetScripts {
    #[serde(rename = "messageID")]
    pub message_id: u8,
}

impl HasId for MessageGetScripts {
    const MESSAGE_ID: u8 = 0;
}

#[derive(Serialize, Debug)]
pub struct MessageReload {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "scriptStates")]
    pub script_states: Value,
}

impl HasId for MessageReload {
    const MESSAGE_ID: u8 = 1;
}

#[derive(Serialize, Debug)]
pub struct MessageExectute {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "returnID")]
    pub return_id: String,
    #[serde(rename = "guid")]
    pub guid: String,
    #[serde(rename = "script")]
    pub script: String,
}

impl HasId for MessageExectute {
    const MESSAGE_ID: u8 = 3;
}

#[derive(Deserialize, Debug)]
pub struct AnswerReload {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "savePath")]
    pub save_path: String,
    #[serde(rename = "scriptStates")]
    pub script_states: Value,
}

impl HasId for AnswerReload {
    const MESSAGE_ID: u8 = 1;
}

impl AnswerReload {
    pub fn get_script_states(&self) -> Result<Value> {
        let script_states = &self.script_states;
        Ok(script_states.clone())
    }
}

#[derive(Deserialize, Debug)]
pub struct AnswerReturn {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "returnID")]
    pub return_id: u8,
    #[serde(rename = "returnValue")]
    pub return_value: Option<String>,
}

impl HasId for AnswerReturn {
    const MESSAGE_ID: u8 = 5;
}

impl AnswerReturn {
    pub fn get_return_value(&self) -> Result<Value> {
        let return_value = &self
            .return_value
            .clone()
            .context("returnValue doesn't exist")?;
        Ok(serde_json::from_str(&return_value)?)
    }
}

/// Get lua scripts
pub fn message_get_lua_scripts() -> Result<AnswerReload> {
    send(&MessageGetScripts {
        message_id: MessageGetScripts::MESSAGE_ID,
    })
}

/// Update the lua scripts and UI XML for any objects listed in the message,
/// and then reload the save file. Objects not mentioned are not updated.
pub fn message_reload(script_states: Value) -> Result<AnswerReload> {
    send(&MessageReload {
        message_id: MessageReload::MESSAGE_ID,
        script_states,
    })
}

/// Executes lua code inside Tabletop Simulator and returns the value.
///
/// This macro uses the same syntax as `format`.
/// The first argument `execute!` receives is a format string. This must be a string literal.
/// To use special characters without escaping use raw string literals.
#[macro_export]
macro_rules! execute {
    ($($arg:tt)+) => {
        message_execute(format!($($arg)+))?
    };
}

/// Executes lua code inside Tabletop Simulator and returns the value.
pub fn message_execute(script: String) -> Result<AnswerReturn> {
    send(&MessageExectute {
        message_id: MessageExectute::MESSAGE_ID,
        return_id: String::from("5"),
        guid: String::from("-1"),
        script,
    })
}
