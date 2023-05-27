use std::fs;
use std::path::{Path, PathBuf};

use crate::print_info;
use inquire::MultiSelect;
use serde_json::{json, Value};
use tts_external_api::ExternalEditorApi;
use ttsst::error::{Error, Result};
use ttsst::reload;
use ttsst::{Object, Save, Tag};

const ATTACH_MESSAGE: &str = "Select the object to attach the script to:";
const DETACH_MESSAGE: &str = "Select the object to detach the script from:";

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: PathBuf, guids: Option<Vec<String>>) -> Result<()> {
    let mut objects = get_objects(api, guids, ATTACH_MESSAGE)?;

    let tag = Tag::from(&path);
    let script = read_file(&path)?;
    // Add tag and script to objects
    for object in objects.iter_mut() {
        object.tags = object.tags.filter_invalid();
        object.tags.push(tag.clone());
        print_info!("added:", "'{tag}' as a tag to {object}");

        object.lua_script = script.clone();
        print_info!("added:", "{path:?} as a script to {object}");
    }

    // Add objects to a new save state
    let mut save = Save::read_save(api)?;
    save.objects.add_objects(&objects)?;

    update_save(api, &save)?;
    Ok(())
}

pub fn detach(api: &ExternalEditorApi, guids: Option<Vec<String>>) -> Result<()> {
    let mut objects = get_objects(api, guids, DETACH_MESSAGE)?;

    // Remove tags and script from objects
    for object in objects.iter_mut() {
        object.tags = object.tags.filter_invalid();
        object.lua_script = String::new();
    }

    // Add objects to a new save state
    let mut save = Save::read_save(api)?;
    save.objects.add_objects(&objects)?;

    update_save(api, &save)?;
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: PathBuf) -> Result<()> {
    let mut save = Save::read_save(api)?;

    // Update the lua script with the file content from the tag
    // Returns Error if the object has multiple valid tags
    for object in save.objects.iter_mut() {
        if let Some(tag) = object.tags.valid()? {
            if (path.is_file() && tag.is_path(&path)) || path.is_dir() {
                object.lua_script = tag.read_file(&path)?;
                print_info!("updated:", "{object} with tag '{tag}'");
            }
        }
    }

    save.lua_script = read_global_files(&path, &["Global.lua", "Global.ttslua"])? // Update lua
        .unwrap_or(save.lua_script); // Fallback to the existing lua script
    save.xml_ui = read_global_files(&path, &["Global.xml"])? // Update xml
        .unwrap_or(save.xml_ui); // Fallback to the existing xml ui

    update_save(api, &save)?;
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
        .map(|guid| save.objects.clone().find_object(&guid))
        .collect() // `Vec<Result<T, E>>` gets turned into `Result<Vec<T>, E>`
}

/// Shows a multi selection prompt of objects loaded in the current save
fn select_objects(save: Save, message: &str) -> Result<Vec<Object>> {
    let objects = save.objects;
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
    let mut objects: Vec<Value> = save
        .objects
        .iter()
        .map(|object| object.to_value())
        .collect();

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

/// Join path and files and filter by existing paths
fn get_existing_paths(path: &Path, files: &[&str]) -> Vec<PathBuf> {
    files
        .iter()
        .map(|file| Path::new(path).join(file)) // concat path and file name
        .filter(|path| path.exists()) // filter by existing paths
        .collect()
}

/// Reads a file from a list of possible file names. Only one of the files can exist on the path, otherwise this
/// function returns an [`Error::Msg`]. If none of the files provided exist `Ok(None)` gets returned.
fn read_global_files(path: &Path, files: &[&str]) -> Result<Option<String>> {
    let paths = get_existing_paths(path, files);
    match paths.len() {
        1 => read_file(&paths[0]).map(Option::Some),
        0 => Ok(None),
        _ => Err(format!("multiple global files exist on the provided path: {paths:?}").into()),
    }
}

/// Reads a file from the path and replaces every occurrence of `\t` with spaces
fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map(|content| content.replace('\t', "    "))
        .map_err(|_| Error::ReadFile)
}
