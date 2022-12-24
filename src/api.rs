//! Reference: https://api.tabletopsimulator.com/externaleditorapi/
//!
//! Communication between the editor and TTS occurs via two localhost TCP connections:
//! one where TTS listens for messages and one where ttsst listens for messages.
//! All communication messages are JSON.

use crate::tcp::send;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

pub trait HasId {
    fn get_message_id() -> u64;
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
    fn get_message_id() -> u64 {
        1
    }
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
    fn get_message_id() -> u64 {
        5
    }
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
    send(
        json!({
            "messageID": 0,
        })
        .to_string(),
    )
}

/// Update the lua scripts and UI XML for any objects listed in the message,
/// and then reload the save file. Objects not mentioned are not updated.
pub fn message_reload(script_states: Value) -> Result<AnswerReload> {
    send(
        json!({
            "messageID": 1,
            "scriptStates": script_states
        })
        .to_string(),
    )
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
    send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": "-1",
            "script": script
        })
        .to_string(),
    )
}
