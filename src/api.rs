//! Reference: https://api.tabletopsimulator.com/externaleditorapi/
//!
//! Communication between the editor and TTS occurs via two localhost TCP connections:
//! one where TTS listens for messages and one where ttsst listens for messages.
//! All communication messages are JSON.
#![allow(dead_code)]

use crate::tcp::send;
use anyhow::Result;
use colorize::AnsiColor;
use serde_json::{json, Value};
use snailquote::unescape;

/// Executes lua code inside Tabletop Simulator and returns the value.
///
/// This macro uses the same syntax as `format`.
/// The first argument `execute!` receives is a format string. This must be a string literal.
/// To use special characters without escaping use raw string literals.
#[macro_export]
macro_rules! execute {
    ($($arg:tt)+) => {
        message_execute(format!($($arg)+))
    };
}

/// Executes lua code inside Tabletop Simulator and returns the value.
pub fn message_execute(code: String) -> Result<Value> {
    let message = send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": "-1",
            "script": code
        })
        .to_string(),
        5,
    )?;
    get_return_value(message)
}

/// Update the lua scripts and UI XML for any objects listed in the message,
/// and then reload the save file. Objects not mentioned are not updated.
pub fn message_reload(script_states: Value) -> Result<Value> {
    let message = send(
        json!({
            "messageID": 1,
            "scriptStates": script_states
        })
        .to_string(),
        1,
    )?;
    println!("{}", "reloaded save!".green().bold());
    Ok(message)
}

/// Sends a message
pub fn send_message(message: &str) -> Result<Value> {
    let message = execute!(
        r#"
            broadcastToAll("{}")
        "#,
        message.escape_default()
    );
    println!("{:?}", message);
    message
}

/// Get lua scripts
pub fn get_lua_scripts() -> Result<Value> {
    send(
        json!({
            "messageID": 0,
        })
        .to_string(),
        1,
    )
}

fn get_return_value(message: Value) -> Result<Value> {
    let unescaped_message = &unescape_value(&message["returnValue"]);
    let result_value: Value = serde_json::from_str(&unescaped_message).unwrap();
    Ok(result_value)
}

/// Unescapes a Value and returns it as a String.
pub fn unescape_value(value: &Value) -> String {
    unescape(&value.to_string()).unwrap()
}
