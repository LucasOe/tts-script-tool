use crate::error::{Error, Result};
use crate::execute;
use crate::script_states::ScriptState;
use std::collections::HashMap;
use tts_external_api::messages::AnswerReload;
use tts_external_api::ExternalEditorApi;

/// Returns a list of all object guids in the current save
pub fn get_objects(api: &ExternalEditorApi) -> Result<Vec<String>> {
    execute!(
        api,
        r#"
            list = {{}}
            for _, obj in pairs(getAllObjects()) do
                table.insert(list, obj.guid)
            end
            return JSON.encode(list)
        "#,
    )
}

/// Returns a list of tags associated with each object in the current save
pub fn get_tag_map(api: &ExternalEditorApi) -> Result<HashMap<String, Vec<String>>> {
    execute!(
        api,
        r#"
            list = {{}}
            for _, obj in pairs(getAllObjects()) do
                list[obj.guid] = obj.getTags()
            end
            return JSON.encode(list)
        "#,
    )
}

/// Returns a list of tags for an object
pub fn get_tags(api: &ExternalEditorApi, guid: &str) -> Result<Vec<String>> {
    execute!(
        api,
        r#"
            return JSON.encode(getObjectFromGUID("{guid}").getTags())
        "#,
    )
}

/// Set the script for an object
pub fn set_script(api: &ExternalEditorApi, guid: &str, script: &str) -> Result<()> {
    let escaped_script = script.escape_default();
    execute!(
        api,
        r#"
            getObjectFromGUID("{guid}").setLuaScript("{escaped_script}")
        "#
    )
}

/// Adds a list of tags to an object
pub fn add_tags(api: &ExternalEditorApi, guid: &str, tags: &Vec<String>) -> Result<()> {
    let tags = serde_json::to_string(tags)?;
    let escaped_tags = tags.escape_default();
    execute!(
        api,
        r#"
            tags = JSON.decode("{escaped_tags}")
            getObjectFromGUID("{guid}").setTags(tags)
        "#
    )
}

/// Get a vec of [`ScriptState`] structs
pub fn get_script_states(api: &ExternalEditorApi) -> Result<Vec<ScriptState>> {
    let script_states = api.get_scripts()?.script_states;
    serde_json::from_value(script_states).map_err(Error::SerdeError)
}

/// Reload save without changing anything
pub fn reload(api: &ExternalEditorApi) -> Result<AnswerReload> {
    api.reload(serde_json::json!([])).map_err(Error::Io)
}

/// Reload save with global script and ui
pub fn reload_global(api: &ExternalEditorApi, script: String, ui: String) -> Result<AnswerReload> {
    api.reload(serde_json::json!(
        [{ "guid": "-1", "script": script, "ui": ui }]
    ))
    .map_err(Error::Io)
}
