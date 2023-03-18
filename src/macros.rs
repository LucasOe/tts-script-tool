/// Executes a lua script globally.
///
/// This macro uses the same format as [`format!`], but the first argument has to be the [`ExternalEditorApi`] used for executing the script.
/// See [`std::fmt`] for more information.
///
/// If the script has no return statement, this macro will return [`Null`].
/// Otherwise the returned value will be deserialized as an instance of type `T` using the [`serde_json::from_value()`] function.
/// If the returned value can't be deserialized, the macro returns an [`Error::SerdeError`].
///
/// Objects have to be JSON encoded using the [`JSON.encode()`](https://api.tabletopsimulator.com/json/#encode) function provided by Tabletop Simulator.
///
/// [`ExternalEditorApi`]: tts_external_api::ExternalEditorApi
/// [`Error::SerdeError`]: crate::error::Error::SerdeError
/// [`Null`]: serde_json::Value#variant.Null
///
/// # Examples
///
/// ```
/// # use ttsst::execute;
/// # use tts_external_api::ExternalEditorApi;
///
/// # fn main() -> ttsst::error::Result<()> {
/// let api = ExternalEditorApi::new();
///
/// execute!(api, r#"print("{}")"#, "Hello, World!")?;
/// let result: f32 = execute!(api, "return 15")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! execute {
    ($api:ident, $($arg:tt)+) => {{
        let result = $api.execute(format!($($arg)*))?.return_value;
        serde_json::from_value(result).map_err($crate::error::Error::SerdeError)
    }}
}

/// Update the Lua scripts and UI XML for any objects listed in the message,
/// and then reloads the save file, the same way it does when pressing “Save & Play” within the in-game editor.
///
/// This macro uses the same format as [`serde_json::json!`], but the first argument has to be the [`ExternalEditorApi`] used for executing the script.
///
/// Any objects mentioned have both their Lua script and their UI XML updated.
/// If no value is set for either the "script" or "ui" key then the
/// corresponding Lua script or UI XML is deleted.
///
/// If no connection to the game can be established, the macro returns an [`Error::Io`].
///
/// [`ExternalEditorApi`]: tts_external_api::ExternalEditorApi
/// [`Error::Io`]: crate::error::Error::Io
///
/// # Examples
///
/// ```
/// # use ttsst::reload;
/// # use tts_external_api::ExternalEditorApi;
///
/// # fn main() -> ttsst::error::Result<()> {
/// let api = ExternalEditorApi::new();
///
/// reload!(api, [{
///     "guid": "-1",
///     "script": "Hello, World!"
/// }])?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! reload {
    ($api:ident, $($arg:tt)+) => {{
        let result = $api.reload(serde_json::json!($($arg)*));
        result.map_err($crate::error::Error::Io)
    }}
}
