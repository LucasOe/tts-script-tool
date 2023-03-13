use crate::error::{Error, Result};
use crate::script_states::ScriptState;
use crate::{messages, print_info};
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

    messages::reload(api)?;
    set_tag(api, file_name, &guid)?;

    print_info!("reloaded save!");
    println!("To save the applied tag it is recommended to save the game before reloading.");
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: &Path) -> Result<()> {
    let guid_tags = match messages::get_tag_map(api) {
        Ok(guid_tags) => Ok(guid_tags),
        Err(_) => Err("The current save has no objects"),
    }?;

    // update scripts with setLuaScript(), so objects without a script get updated.
    for (guid, tags) in guid_tags {
        let (tags, _) = partition_tags(tags);
        // ensure that the object only has one valid tag
        let valid_tag: Option<String> = match tags.len() {
            1 => Some(tags[0].clone()),
            0 => None,
            _ => return Err("{guid} has multiple valid script tags: {tags:?}".into()),
        };

        if let Some(tag) = valid_tag {
            let file_path = get_file_from_tag(path, &tag);
            let file_content = fs::read_to_string(file_path)?;
            messages::set_script(api, &guid, &file_content)?;
            print_info!("updated:", "'{guid}' with tag '{tag}'");
        }
    }

    // Get global script and ui and reload the current save
    let script_states = messages::get_script_states(api)?;
    let script_state = script_states.get(0).unwrap();
    let global_script = get_global_script(path, script_state)?;
    let global_ui = get_global_ui(path, script_state)?;

    messages::reload_global(api, global_script, global_ui)?;

    print_info!("reloaded save!");
    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, path: &Path) -> Result<()> {
    let save_path = PathBuf::from(api.get_scripts()?.save_path);
    let mut path = PathBuf::from(path);
    path.set_extension("json");
    fs::copy(&save_path, &path)?;
    backup_print(&save_path, &path);
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
// Guid has to be global so objects without scripts can execute code.
fn set_tag(api: &ExternalEditorApi, file_name: &str, guid: &str) -> Result<String> {
    // get existing tags for object
    let tag = format!("scripts/{file_name}");
    let tags = messages::get_tags(api, guid)?;

    // set new tags for object
    let (_, mut tags) = partition_tags(tags);
    tags.push(String::from(&tag));
    messages::add_tags(api, guid, &tags)?;

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

/// Print information for the backup function
fn backup_print(save_path: &Path, path: &Path) {
    let save_name_str = Path::new(&save_path).file_name().unwrap().to_str().unwrap();
    let path_str = path.to_str().unwrap();
    print_info!("save:", "'{save_name_str}' as '{path_str}'");
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
        (false, false) => Ok(script_state.clone().script()),
    }
}

/// Get a global ui from a file or get the ui from the current save if no file exists.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_ui(path: &Path, script_state: &ScriptState) -> Result<String> {
    let global_xml = Path::new(path).join("./Global.xml");
    match global_xml.exists() {
        true => fs::read_to_string(global_xml).map_err(|_| Error::ReadFile),
        false => Ok(script_state.clone().ui()),
    }
}
