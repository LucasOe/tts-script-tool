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

/////////////////////////////////////////////////////////////////////////////

/// Get a list containing the states for every object. Returns an `AnswerReload` message.
#[derive(Serialize, Debug, PartialEq)]
pub struct MessageGetScripts {
    #[serde(rename = "messageID")]
    pub message_id: u8,
}

impl HasId for MessageGetScripts {
    const MESSAGE_ID: u8 = 0;
}

/// Update the Lua scripts and UI XML for any objects listed in the message,
/// and then reloads the save file, the same way it does when pressing "Save & Play" within the in-game editor.
/// Returns an `AnswerReload` message.
///
/// Any objects mentioned have both their Lua script and their UI XML updated.
/// If no value is set for either the "script" or "ui" key then the
/// corresponding Lua script or UI XML is deleted.
#[derive(Serialize, Debug, PartialEq)]
pub struct MessageReload {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "scriptStates")]
    pub script_states: Value,
}

impl HasId for MessageReload {
    const MESSAGE_ID: u8 = 1;
}

/// Send a custom message to be forwarded to the `onExternalMessage` event handler
/// in the currently loaded game. The value of customMessage must be a table,
/// and is passed as a parameter to the event handler.
/// If this value is not a table then the event is not triggered.
#[derive(Serialize, Debug, PartialEq)]
pub struct MessageCustomMessage {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "customMessage")]
    pub custom_message: Value,
}

impl HasId for MessageCustomMessage {
    const MESSAGE_ID: u8 = 2;
}

/// Executes a lua script and returns the value in a `AnswerReturn` message.
/// Using a guid of "-1" runs the script globally.
#[derive(Serialize, Debug, PartialEq)]
pub struct MessageExectute {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "returnID")]
    pub return_id: u8,
    #[serde(rename = "guid")]
    pub guid: String,
    #[serde(rename = "script")]
    pub script: String,
}

impl HasId for MessageExectute {
    const MESSAGE_ID: u8 = 3;
}

/////////////////////////////////////////////////////////////////////////////

/// When clicking on "Scripting Editor" in the right click contextual menu
/// in TTS for an object that doesn't have a Lua Script yet, TTS will send
/// an `AnswerNewObject` message containing data for the object.
///
/// # Example
/// ```json
/// {
///     "message_id": 0,
///     "script_states": [
///         {
///             "name": "Chess Pawn",
///             "guid": "db3f06",
///             "script": ""
///         }
///     ]
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerNewObject {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "scriptStates")]
    pub script_states: Value,
}

impl HasId for AnswerNewObject {
    const MESSAGE_ID: u8 = 0;
}

/// After loading a new game in TTS, TTS will send all the Lua scripts
/// and UI XML from the new game as an `AnswerReload`.
///
/// TTS sends this message as a response to `MessageGetScripts` and `MessageReload`.
///
/// # Example
/// ```json
/// {
///     "message_id": 1,
///     "script_states": [
///         {
///             "name": "Global",
///             "guid": "-1",
///             "script": "...",
///             "ui": "..."
///         },
///         {
///             "name": "BlackJack Dealer's Deck",
///             "guid": "a0b2d5",
///             "script": "..."
///         },
///     ]
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
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

/// TTS sends all `print()` messages in a `AnswerPrint` response.
///
/// # Example
/// ```json
/// {
///     "message_id": 2,
///     "message": "Hit player! White"
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerPrint {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "message")]
    pub message: String,
}

impl HasId for AnswerPrint {
    const MESSAGE_ID: u8 = 2;
}

/// TTS sends all error messages in a `AnswerError` response.
///
/// # Example
/// ```json
/// {
///     "message_id": 3,
///     "error": "chunk_0:(36,4-8): unexpected symbol near 'deck'",
///     "guid": "-1",
///     "errorMessagePrefix": "Error in Global Script: "
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerError {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "guid")]
    pub guid: String,
    #[serde(rename = "errorMessagePrefix")]
    pub error_message_prefix: String,
}

impl HasId for AnswerError {
    const MESSAGE_ID: u8 = 3;
}

/// Custom Messages are sent by calling `sendExternalMessage` with the table of data you wish to send.
///
/// # Example
/// ```json
/// {
///     "message_id": 4,
///     "custom_message": { "foo": "Hello", "bar": "World"}
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerCustomMessage {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "customMessage")]
    pub custom_message: Value,
}

impl HasId for AnswerCustomMessage {
    const MESSAGE_ID: u8 = 4;
}

/// If code executed with a `MessageExecute` message returns a value,
/// it will be sent back in a `AnswerReturn` message.
///
/// Return values can only be strings. Tables have to be decoded using `JSON.decode(table)`.
///
/// # Example
/// ```json
/// {
///     "message_id": 5,
///     "return_value": true
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
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

/// Whenever the player saves the game in TTS, `AnswerGameSaved` is sent as a response.
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerGameSaved {
    #[serde(rename = "messageID")]
    pub message_id: u8,
}

impl HasId for AnswerGameSaved {
    const MESSAGE_ID: u8 = 6;
}

/// Whenever the player saves the game in TTS, `AnswerObjectCreated` is sent as a response.
///
/// # Example
/// ```json
/// {
///     "message_id": 7,
///     "guid": "abcdef"
/// }
/// ```
#[derive(Deserialize, Debug, PartialEq)]
pub struct AnswerObjectCreated {
    #[serde(rename = "messageID")]
    pub message_id: u8,
    #[serde(rename = "guid")]
    pub guid: String,
}

impl HasId for AnswerObjectCreated {
    const MESSAGE_ID: u8 = 7;
}

/////////////////////////////////////////////////////////////////////////////

pub fn message_get_lua_scripts() -> Result<AnswerReload> {
    send(&MessageGetScripts {
        message_id: MessageGetScripts::MESSAGE_ID,
    })
}

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

pub fn message_execute(script: String) -> Result<AnswerReturn> {
    send(&MessageExectute {
        message_id: MessageExectute::MESSAGE_ID,
        return_id: 5,
        guid: String::from("-1"),
        script,
    })
}

/////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::api::*;

    #[test]
    fn test_execute() {
        let script = String::from(
            r#"
                return JSON.encode("5")
            "#,
        );
        let answer: AnswerReturn = send(&MessageExectute {
            message_id: MessageExectute::MESSAGE_ID,
            return_id: 5,
            guid: String::from("-1"),
            script,
        })
        .unwrap();
        let expected_answer = AnswerReturn {
            message_id: AnswerReturn::MESSAGE_ID,
            return_id: 5,
            return_value: Some("\"5\"".to_string()),
        };
        assert_eq!(answer, expected_answer);
    }
}
