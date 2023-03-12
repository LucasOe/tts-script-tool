use crate::error::{Error, Result};
use crate::{messages::*, print_info};
use colorize::AnsiColor;
use inquire::Select;
use regex::Regex;
use snailquote::unescape;
use std::fs;
use std::path::{Path, PathBuf};
use tts_external_api::{json, ExternalEditorApi};

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: &PathBuf, guid: Option<String>) -> Result<()> {
    let path = Path::new(path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let guid = get_guid(api, guid)?;
    let tag = set_tag(api, file_name, &guid)?;
    print_info!("added:", "'{tag}' as a tag for '{guid}'");

    let file_content = fs::read_to_string(path)?;
    set_script(api, &guid, &file_content)?;
    print_info!("updated:", "'{guid}' with tag '{tag}'");

    api.reload(json!([]))?;
    set_tag(api, file_name, &guid)?;

    print_info!("reloaded save!");
    println!("To save the applied tag it is recommended to save the game before reloading.");
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: &PathBuf) -> Result<()> {
    let guid_tags = get_tag_map(&api)?;

    // update scripts with setLuaScript(), so objects without a script get updated.
    for (guid, tags) in guid_tags {
        let (tags, _) = partition_tags(tags);
        // ensure that the object only has one valid tag
        let valid_tag: Option<String> = match tags.len() {
            1 => Some(tags[0].clone()),
            0 => None,
            _ => return Err(Error::ValidTags { guid, tags }),
        };

        if let Some(tag) = valid_tag {
            let file_path = get_file_from_tag(path, &tag);
            let file_content = fs::read_to_string(file_path)?;
            set_script(api, &guid, &file_content)?;
            print_info!("updated:", "'{guid}' with tag '{tag}'");
        }
    }

    // get scriptStates
    let save_data = api.get_scripts()?.script_states;
    let script_states = save_data.as_array().unwrap();

    // add global script to script_list
    let global_path = Path::new(path).join("./Global.ttslua");

    // get global script from file and fallback to existing script the from save data
    let global_script = match fs::read_to_string(global_path) {
        Ok(global_file_content) => global_file_content,
        Err(_) => unescape(&script_states[0].get("script").unwrap().to_string()).unwrap(),
    };
    // get global ui from the save data
    let global_ui = unescape(&script_states[0].get("ui").unwrap().to_string()).unwrap();

    api.reload(json!([{
        "guid": "-1",
        "script": global_script,
        "ui": global_ui
    }]))?;

    print_info!("reloaded save!");
    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, path: &PathBuf) -> Result<()> {
    let mut path = PathBuf::from(path);
    path.set_extension("json");
    let save_path = api.get_scripts()?.save_path;
    fs::copy(&save_path, &path)?;
    print_info!(
        "save:",
        "'{}' as '{}'",
        Path::new(&save_path).file_name().unwrap().to_str().unwrap(),
        path.to_str().unwrap()
    );
    Ok(())
}

/// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn set_tag(api: &ExternalEditorApi, file_name: &str, guid: &str) -> Result<String> {
    // get existing tags for object
    let tag = format!("scripts/{file_name}");
    let tags = get_tags(api, guid)?;

    // set new tags for object
    let (_, mut tags) = partition_tags(tags);
    tags.push(String::from(&tag));
    add_tags(api, guid, &tags)?;

    Ok(tag)
}

/// Split the tags into valid and invalid tags.
/// Tags that follow the "scripts/<File>.ttslua" naming convention are valid.
fn partition_tags(tags: Vec<String>) -> (Vec<String>, Vec<String>) {
    let exprs = Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
    tags.into_iter().partition(|tag| exprs.is_match(tag))
}

/// Gets the corresponding from the path according to the tag. Path has to be a directory.
fn get_file_from_tag(path: &PathBuf, tag: &str) -> String {
    let path = Path::new(path);
    let file_name = Path::new(&tag).file_name().unwrap();
    let file_path = path.join(file_name);
    String::from(file_path.to_string_lossy())
}

/// If no guid is provided show a selection of objects in the current save.
/// Otherwise ensure that the guid provided exists. Returns [`Error::MissingGuid`] if it does not exist.
fn get_guid(api: &ExternalEditorApi, guid: Option<String>) -> Result<String> {
    match guid {
        Some(guid) => guid_exists(api, guid),
        None => select_object(api),
    }
}

/// Returns [`Error::MissingGuid`] if the guid doesn't exist in the current save
fn guid_exists(api: &ExternalEditorApi, guid: String) -> Result<String> {
    match get_objects(api)?.contains(&guid) {
        true => Ok(guid),
        false => Err(Error::MissingGuid(guid)),
    }
}

/// Shows the user a list of all objects in the save to select from
fn select_object(api: &ExternalEditorApi) -> Result<String> {
    let objects = get_objects(api)?;
    Select::new("Select the object to attach the script to:", objects)
        .prompt()
        .map_err(Error::InquireError)
}
