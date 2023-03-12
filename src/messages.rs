use crate::error::Result;
use crate::execute;
use std::collections::HashMap;
use tts_external_api::ExternalEditorApi;

/// Sets the script for the object.
pub fn set_script(api: &ExternalEditorApi, guid: &str, script: &str) -> Result<()> {
    execute!(
        api,
        r#"
            getObjectFromGUID("{guid}").setLuaScript("{}")
        "#,
        script.escape_default()
    )
}

/// Returns a list of all guids
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

/// Returns a list of tags associated with each object
pub fn get_tags(api: &ExternalEditorApi) -> Result<HashMap<String, Vec<String>>> {
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
