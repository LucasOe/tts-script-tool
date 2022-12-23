//! Reference: https://api.tabletopsimulator.com/externaleditorapi/
//!
//! Communication between the editor and TTS occurs via two localhost TCP connections:
//! one where TTS listens for messages and one where ttsst listens for messages.
//! All communication messages are JSON.

use crate::tcp::send;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug)]
pub enum Answer {
    AnswerReload(AnswerReload),
    AnswerReturn(AnswerReturn),
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

#[derive(Deserialize, Debug)]
pub struct AnswerReturn {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "returnID")]
    pub return_id: u8,
    #[serde(rename = "returnValue")]
    pub return_value: Option<String>,
}

impl AnswerReload {
    pub fn get_script_states(&self) -> Result<Value> {
        let script_states = &self.script_states;
        Ok(script_states.clone())
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
    let answer = send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": "-1",
            "script": script
        })
        .to_string(),
        5,
    )?;
    if let Answer::AnswerReturn(answer_return) = answer {
        return Ok(answer_return);
    }
    bail!("Message couldn't be deserialized into an AnswerReturn");
}

/// Update the lua scripts and UI XML for any objects listed in the message,
/// and then reload the save file. Objects not mentioned are not updated.
pub fn message_reload(script_states: Value) -> Result<AnswerReload> {
    let answer = send(
        json!({
            "messageID": 1,
            "scriptStates": script_states
        })
        .to_string(),
        1,
    )?;
    if let Answer::AnswerReload(answer_reload) = answer {
        return Ok(answer_reload);
    }
    bail!("Message couldn't be deserialized into an AnswerReturn");
}

/// Get lua scripts
pub fn message_get_lua_scripts() -> Result<AnswerReload> {
    let answer = send(
        json!({
            "messageID": 0,
        })
        .to_string(),
        1,
    )?;
    if let Answer::AnswerReload(answer_reload) = answer {
        return Ok(answer_reload);
    }
    bail!("Message couldn't be deserialized into an AnswerReturn");
}
