use crate::error::{Error, Result};
use crate::script_states::{ScriptState, ScriptStates};
use crate::{messages, print_info, reload_save};
use inquire::Select;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use tts_external_api::ExternalEditorApi;

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: &Path, guid: Option<String>) -> Result<()> {
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let guid = get_guid(api, guid)?;
    let tag = set_tag(api, file_name, &guid)?;
    print_info!("added:", "'{tag}' as a tag for '{guid}'");

    let file_content = fs::read_to_string(path)?;
    messages::set_script(api, &guid, &file_content)?;
    print_info!("updated:", "'{guid}' with tag '{tag}'");

    reload_save!(api, [])?;
    set_tag(api, file_name, &guid)?;

    print_info!("reloaded save!");
    println!("To save the applied tag it is recommended to save the game before reloading.");
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: &Path) -> Result<()> {
    let tags_map = match messages::get_tag_map(api) {
        Ok(guid_tags) => Ok(guid_tags),
        Err(_) => Err("The current save has no objects"),
    }?;

    for (guid, tags) in tags_map {
        let (tags, _) = partition_tags(tags);
        // Ensure that the object only has one valid tag
        let valid_tag = match tags.len() {
            1 => tags.get(0),
            0 => None,
            _ => return Err("{guid} has multiple valid script tags: {tags:?}".into()),
        };

        // Update the script with the file content from the tag
        if let Some(tag) = valid_tag {
            let file_path = get_file_from_tag(path, tag);
            let file_content = fs::read_to_string(file_path)?;
            // Update scripts with `setLuaScript()` instead of using `api.reload()`,
            // so objects without a script get updated.
            messages::set_script(api, &guid, &file_content)?;
            print_info!("updated:", "'{guid}' with tag '{tag}'");
        }
    }

    // Get global script and ui and reload the current save.
    // Gets the global script and ui from files on the provided path first.
    // If no files exist, it uses the script and the ui from the current save.
    let script_states = ScriptStates::new(api)?;
    let script_state = &script_states.global().unwrap();
    reload_save!(
        api,
        [{
            "guid": "-1",
            "script": get_global_script(path, script_state)?,
            "ui": get_global_ui(path, script_state)?
        }]
    )?;
    print_info!("reloaded save!");

    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, path: &Path) -> Result<()> {
    let save_path = PathBuf::from(api.get_scripts()?.save_path);
    let mut path = PathBuf::from(path);
    path.set_extension("json");
    fs::copy(&save_path, &path)?;

    // Print information about the file
    let save_name_str = Path::new(&save_path).file_name().unwrap().to_str().unwrap();
    let path_str = path.to_str().unwrap();
    print_info!("save:", "'{save_name_str}' as '{path_str}'");

    Ok(())
}

/// If no guid is provided show a selection of objects in the current save.
/// Otherwise ensure that the guid provided exists. Returns [`Error::MissingGuid`] if it does not exist.
fn get_guid(api: &ExternalEditorApi, guid: Option<String>) -> Result<String> {
    let objects = messages::get_objects(api)?;
    match guid {
        Some(guid) => guid_exists(objects, guid),
        None => select_guid(objects),
    }
}

/// Returns [`Error::MissingGuid`] if the guid doesn't exist in the current save
fn guid_exists(objects: Vec<String>, guid: String) -> Result<String> {
    match objects.contains(&guid) {
        true => Ok(guid),
        false => Err(format!("{guid} does not exist").into()),
    }
}

/// Shows the user a list of all objects in the save to select from
fn select_guid(objects: Vec<String>) -> Result<String> {
    Select::new("Select the object to attach the script to:", objects)
        .prompt()
        .map_err(Error::InquireError)
}

/// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
fn set_tag(api: &ExternalEditorApi, file_name: &str, guid: &str) -> Result<String> {
    let tag = format!("scripts/{file_name}");

    // Set new script tag for object and add the previous invalid tags.
    // Previous valid tags will be overwritten with the new script tag.
    let tags = messages::get_tags(api, guid)?;
    let (_, mut tags) = partition_tags(tags);
    tags.push(String::from(&tag));
    messages::set_tags(api, guid, &tags)?;

    Ok(tag)
}

/// Split the tags into valid and invalid tags.
/// Tags that follow the "scripts/<File>.ttslua" naming convention are valid.
fn partition_tags(tags: Vec<String>) -> (Vec<String>, Vec<String>) {
    let exprs = Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
    tags.into_iter().partition(|tag| exprs.is_match(tag))
}

/// Gets the corresponding from the path according to the tag. Path has to be a directory.
fn get_file_from_tag(path: &Path, tag: &str) -> String {
    let file_name = Path::new(&tag).file_name().unwrap();
    String::from(path.join(file_name).to_string_lossy())
}

/// Get a global script from a file or get the script from the current save if no file exists.
/// Returns [`Error::Msg`] if both "Global.ttslua" and "Global.lua" exist.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_script(path: &Path, script_state: &ScriptState) -> Result<String> {
    let global_tts = Path::new(path).join("./Global.ttslua");
    let global_lua = Path::new(path).join("./Global.lua");
    match (global_tts.exists(), global_lua.exists()) {
        (true, true) => Err("Global.ttslua and Global.lua both exist on the provided path".into()),
        (true, false) => fs::read_to_string(global_tts).map_err(|_| Error::ReadFile),
        (false, true) => fs::read_to_string(global_lua).map_err(|_| Error::ReadFile),
        (false, false) => Ok(script_state.script()),
    }
}

/// Get a global ui from a file or get the ui from the current save if no file exists.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_ui(path: &Path, script_state: &ScriptState) -> Result<String> {
    let global_xml = Path::new(path).join("./Global.xml");
    match global_xml.exists() {
        true => fs::read_to_string(global_xml).map_err(|_| Error::ReadFile),
        false => Ok(script_state.ui()),
    }
}
