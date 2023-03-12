use crate::error::Result;
use crate::execute;
use std::collections::HashMap;
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
                if obj.hasAnyTag() then
                    list[obj.guid] = obj.getTags()
                end
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
