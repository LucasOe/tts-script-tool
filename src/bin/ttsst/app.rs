use std::fs;
use std::path::{Path, PathBuf};

use crate::print_info;
use inquire::MultiSelect;
use serde_json::{json, Value};
use tts_external_api::ExternalEditorApi;
use ttsst::error::{Error, Result};
use ttsst::reload;
use ttsst::{Object, Save, Tag};

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: PathBuf, guids: Option<Vec<String>>) -> Result<()> {
    let mut objects = get_objects(api, guids, "Select the object to attach the script to:")?;

    let tag = Tag::from(&path);
    let script = read_file(&path)?;
    // Add tag and script to objects
    for mut object in &mut objects {
        let mut new_tags = object.tags.clone().filter_invalid();
        new_tags.push(tag.clone());
        object.tags = new_tags;
        print_info!("added:", "'{tag}' as a tag to {object}");

        object.lua_script = script.clone();
        print_info!("added:", "{path:?} as a script to {object}");
    }

    // Add objects to a new save state
    let mut save_state = Save::read_save(api)?;
    save_state.object_states.add_objects(&objects)?;

    update_save(api, &save_state)?;
    Ok(())
}

pub fn detach(api: &ExternalEditorApi, guids: Option<Vec<String>>) -> Result<()> {
    let mut objects = get_objects(api, guids, "Select the object to detach the script from:")?;

    // Remove tags and script from objects
    for mut object in &mut objects {
        let new_tags = object.tags.clone().filter_invalid();
        object.tags = new_tags;
        object.lua_script = String::new();
    }

    // Add objects to a new save state
    let mut save_state = Save::read_save(api)?;
    save_state.object_states.add_objects(&objects)?;

    update_save(api, &save_state)?;
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: PathBuf) -> Result<()> {
    let mut save_state = Save::read_save(api)?;

    // Update the lua script with the file content from the tag
    // Returns Error if the object has multiple valid tags
    for mut object in save_state.object_states.iter_mut() {
        if let Some(tag) = object.tags.clone().valid()? {
            if (path.is_file() && tag.is_path(&path)) || path.is_dir() {
                object.lua_script = tag.read_file(&path)?;
                print_info!("updated:", "{object} with tag '{tag}'");
            }
        }
    }

    // Get global script and ui from the files provided on the path.
    // If no files exist, fallback to the save state from the current save.
    save_state.lua_script = get_global_script(&path, &save_state)?;
    save_state.xml_ui = get_global_ui(&path, &save_state)?;

    update_save(api, &save_state)?;
    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, mut path: PathBuf) -> Result<()> {
    let save_path = api.get_scripts()?.save_path;
    path.set_extension("json");
    fs::copy(&save_path, &path)?;

    // Print information about the file
    let save_name_str = Path::new(&save_path).file_name().unwrap().to_str().unwrap();
    let path_str = path.to_str().unwrap();
    print_info!("save:", "'{save_name_str}' as '{path_str}'");

    Ok(())
}

/// If no guids are provided show a selection of objects in the current savestate.
/// Otherwise ensure that the guids provided exist.
fn get_objects(
    api: &ExternalEditorApi,
    guids: Option<Vec<String>>,
    message: &str,
) -> Result<Vec<Object>> {
    let save = Save::read_save(api)?;
    match guids {
        Some(guids) => validate_guids(save, guids),
        None => select_objects(save, message),
    }
}

/// Once an `Result::Err` is found, the iteration will terminate and return the result.
/// If `guids` only contains existing objects, a vec with the savestate of those objects will be returned.
fn validate_guids(save: Save, guids: Vec<String>) -> Result<Vec<Object>> {
    guids
        .into_iter()
        .map(|guid| save.object_states.clone().find_object(&guid))
        .collect() // `Vec<Result<T, E>>` gets turned into `Result<Vec<T>, E>`
}

/// Shows a multi selection prompt of objects loaded in the current save
fn select_objects(save: Save, message: &str) -> Result<Vec<Object>> {
    let objects = save.object_states;
    MultiSelect::new(message, objects.into_inner())
        .prompt()
        .map_err(Error::InquireError)
}

/// Overwrite the save file and reload the current save,
/// the same way it get reloaded when pressing “Save & Play” within the in-game editor.
fn update_save(api: &ExternalEditorApi, save: &Save) -> Result<()> {
    // Overwrite the save file with the modified objects
    save.write_save(api)?;

    // Map every `Object` in the `save` to `serde_json::Value`
    let mut objects = save
        .object_states
        .iter()
        .map(|object| object.to_value())
        .collect::<Vec<Value>>();

    // Add global script and ui to `objects`
    objects.push(json!({
        "guid": "-1",
        "script": save.lua_script,
        "ui": save.xml_ui
    }));

    reload!(api, objects)?;
    print_info!("reloaded save!");
    Ok(())
}

/// Get a global script from a file or fallback to the save state from the current save if no file exists.
/// Returns [`Error::Msg`] if both "Global.ttslua" and "Global.lua" exist.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_script(path: &Path, save_state: &Save) -> Result<String> {
    let global_tts = Path::new(path).join("./Global.ttslua");
    let global_lua = Path::new(path).join("./Global.lua");
    match (global_tts.exists(), global_lua.exists()) {
        (true, true) => Err("Global.ttslua and Global.lua both exist on the provided path".into()),
        (true, false) => read_file(&global_tts),
        (false, true) => read_file(&global_lua),
        (false, false) => Ok(save_state.lua_script.clone()),
    }
}

/// Get a global ui from a file or fallback to the save state from the current save if no file exists.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_ui(path: &Path, save_state: &Save) -> Result<String> {
    let global_xml = Path::new(path).join("./Global.xml");
    match global_xml.exists() {
        true => read_file(&global_xml),
        false => Ok(save_state.xml_ui.clone()),
    }
}

/// Reads a file from the path and replaces every occurrence of `\t` with spaces
fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map(|content| content.replace('\t', "    "))
        .map_err(|_| Error::ReadFile)
}
