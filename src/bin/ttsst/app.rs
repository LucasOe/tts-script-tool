use std::fs;
use std::path::{Path, PathBuf};

use crate::print_info;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;
use ttsst::{Objects, Save, Tag};

const ATTACH_MESSAGE: &str = "Select the object to attach the script to:";
const DETACH_MESSAGE: &str = "Select the object to detach the script from:";

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(
    api: &ExternalEditorApi,
    path: PathBuf,
    guids: Option<Vec<String>>,
    show_all: bool,
) -> Result<()> {
    let mut objects = get_objects(api, guids, show_all, ATTACH_MESSAGE)?;

    let tag = Tag::from(&path);
    let script = read_file(&path)?;
    // Add tag and script to objects
    for object in objects.iter_mut() {
        object.tags.retain(|tag| !tag.is_valid());
        object.tags.push(tag.clone());
        print_info!("added:", "'{tag}' as a tag to {object}");

        object.lua_script = script.clone();
        print_info!("added:", "{path:?} as a script to {object}");
    }

    // Add objects to a new save state
    let mut save = Save::read_save(api)?;
    save.objects.replace(&mut objects);

    update_save(api, &save)?;
    Ok(())
}

pub fn detach(api: &ExternalEditorApi, guids: Option<Vec<String>>, show_all: bool) -> Result<()> {
    let mut objects = get_objects(api, guids, show_all, DETACH_MESSAGE)?;

    // Remove tags and script from objects
    for object in objects.iter_mut() {
        object.tags.retain(|tag| !tag.is_valid());
        object.lua_script = String::new();
    }

    // Add objects to a new save state
    let mut save = Save::read_save(api)?;
    save.objects.replace(&mut objects);

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
                object.lua_script = read_file(&tag.join_path(&path)?)?;
                print_info!("updated:", "{object} with tag '{tag}'");
            }
        }
    }

    update_global(&mut save, &path)?;
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
    show_all: bool,
    message: &str,
) -> Result<Objects> {
    let save = Save::read_save(api)?;
    match guids {
        Some(guids) => validate_guids(save, guids),
        None => select_objects(save, message, show_all),
    }
}

/// Once an `Result::Err` is found, the iteration will terminate and return the result.
/// If `guids` only contains existing objects, a vec with the savestate of those objects will be returned.
fn validate_guids(save: Save, guids: Vec<String>) -> Result<Objects> {
    guids
        .into_iter()
        .map(|guid| save.objects.clone().find_object(&guid))
        .collect() // `Vec<Result<T, E>>` gets turned into `Result<Vec<T>, E>`
}

/// Shows a multi selection prompt of objects loaded in the current save
fn select_objects(save: Save, message: &str, show_all: bool) -> Result<Objects> {
    let objects = match show_all {
        true => save.objects,
        false => save.objects.filter_hidden(),
    };

    match inquire::MultiSelect::new(message, objects.into_inner()).prompt() {
        Ok(obj) => Ok(obj.into()),
        Err(err) => Err(err.into()),
    }
}

/// Overwrite the save file and reload the current save,
/// the same way it get reloaded when pressing “Save & Play” within the in-game editor.
fn update_save(api: &ExternalEditorApi, save: &Save) -> Result<()> {
    // Overwrite the save file with the modified objects
    save.write_save(api)?;

    // Add global lua_script and xml_ui to save
    let mut objects = save.objects.to_values();
    objects.push(serde_json::json!({
        "guid": "-1",
        "script": save.lua_script,
        "ui": save.xml_ui,
    }));

    // Reload save
    api.reload(serde_json::json!(objects))?;
    print_info!("reloaded save!");
    Ok(())
}

/// Set the lua script of the save to either `Global.lua` or `Global.ttslua`, if one of them exists in the `path` directory.
/// Set the xml ui of the save to `Global.xml`, if it exists in the `path` directory.
/// Returns an error if multiple files exist in the `path` directory.
fn update_global(save: &mut Save, path: &Path) -> Result<()> {
    const GLOBAL_LUA: &[&str] = &["Global.lua", "Global.ttslua"];
    const GLOBAL_XML: &[&str] = &["Global.xml"];

    // Update lua_script
    if let Some(path) = get_global_path(path, GLOBAL_LUA)? {
        let file = read_file(&path)?;
        save.lua_script = match file.is_empty() {
            #[rustfmt::skip]
            true => "--[[ Lua code. See documentation: https://api.tabletopsimulator.com/ --]]".to_string(),
            false => file,
        };
    };

    // Update xml_ui
    if let Some(path) = get_global_path(path, GLOBAL_XML)? {
        let file: String = read_file(&path)?;
        save.xml_ui = match file.is_empty() {
            #[rustfmt::skip]
            true => "<!-- Xml UI. See documentation: https://api.tabletopsimulator.com/ui/introUI/ -->".to_string(),
            false => file,
        };
    };

    Ok(())
}

/// Returns a path to a global script, if `paths` only contains a single existing file inside the directory.
/// If it contains no existing files it returns [`None`].
/// If `files` contains multiple existing files, this function returns an [`Error::Msg`].
fn get_global_path(path: &Path, files: &[&str]) -> Result<Option<PathBuf>> {
    // Get a vec of existing file paths for `files` in the `path` directory.
    let paths: Vec<PathBuf> = files
        .iter()
        .map(|file| Path::new(path).join(file))
        .filter(|path| path.exists())
        .collect();

    match paths.len() {
        0 | 1 => Ok(paths.get(0).cloned()),
        _ => Err("multiple files for the global script exist".into()),
    }
}

/// Reads a file from the path and replaces every occurrence of `\t` with spaces.
fn read_file(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content.replace('\t', "    ")),
        Err(err) => Err(err.into()),
    }
}
