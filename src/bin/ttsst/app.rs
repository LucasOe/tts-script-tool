use std::fs;
use std::path::{Path, PathBuf};

use crate::Guids;
use colored::Colorize;
use log::*;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;
use ttsst::{Objects, Save, Tag};

pub enum Mode {
    Attach,
    Detach,
}

impl Mode {
    pub fn msg(&self) -> &str {
        match self {
            Mode::Attach => "Select the object to attach the script or ui element to:",
            Mode::Detach => "Select the object to detach the script and ui element from:",
        }
    }
}

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: PathBuf, guids: Guids) -> Result<()> {
    let mut objects = get_objects(api, guids, Mode::Attach)?;

    let tag = Tag::try_from(path.as_path())?;
    let file = read_file(&path)?;
    for object in objects.iter_mut() {
        // Add lua tag to objects
        if tag.is_lua() {
            object.tags.retain(|tag| !tag.is_lua());
            object.tags.push(tag.clone());
            object.lua_script = file.clone();
            info!("added {tag} as a script to {object}");
        }
        // Add xml tag to objects
        if tag.is_xml() {
            object.tags.retain(|tag| !tag.is_xml());
            object.tags.push(tag.clone());
            object.xml_ui = file.clone();
            info!("added {tag} as a ui element to {object}");
        }
    }

    // Add objects to a new save state
    let mut save = Save::read(api)?;
    save.objects.replace(&mut objects);

    update_save(api, &save)?;
    Ok(())
}

pub fn detach(api: &ExternalEditorApi, guids: Guids) -> Result<()> {
    let mut objects = get_objects(api, guids, Mode::Detach)?;

    // Remove tags and script from objects
    for object in objects.iter_mut() {
        object.tags.retain(|tag| !tag.is_valid());
        object.lua_script = String::new();
    }

    // Add objects to a new save state
    let mut save = Save::read(api)?;
    save.objects.replace(&mut objects);

    update_save(api, &save)?;
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, paths: Vec<PathBuf>) -> Result<()> {
    let mut save = Save::read(api)?;

    for path in &paths {
        for object in save.objects.iter_mut() {
            // Update lua scripts if the path is a lua file
            match object.valid_lua()? {
                Some(tag) => {
                    if tag.starts_with(&path) {
                        object.lua_script = read_file(&tag.path()?)?;
                        info!("updated {object} with tag {tag}");
                    }
                }
                // Remove lua script if the objects has no valid tag
                None => {
                    if !object.lua_script.is_empty() {
                        object.lua_script = "".to_string();
                        info!("removed lua script from {}", object);
                    }
                }
            }
            // Update xml ui if the path is a xml file
            match object.valid_xml()? {
                Some(tag) => {
                    if tag.starts_with(&path) {
                        object.xml_ui = read_file(&tag.path()?)?;
                        info!("updated {object} with tag {tag}");
                    }
                }
                // Remove xml ui if the objects has no valid tag
                None => {
                    if !object.xml_ui.is_empty() {
                        object.xml_ui = "".to_string();
                        info!("removed xml ui from {}", object);
                    }
                }
            }
        }
    }

    // Use the first path that is a dir to search for global files
    // TODO: Support Global files in subdirectories: Add `-g` flag or search recursively
    if let Some(path) = &paths.into_iter().find(|path| path.is_dir()) {
        update_global(&mut save, path)?;
    }

    update_save(api, &save)?;
    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, path: PathBuf) -> Result<()> {
    let save_path = api.get_scripts()?.save_path;
    fs::copy(&save_path, &path)?;

    // Print information about the file
    let save_name = Path::new(&save_path).file_name().unwrap().to_str().unwrap();
    info!("save '{}' as '{}'", save_name, path.display());

    Ok(())
}

/// If no guids are provided show a selection of objects in the current savestate.
/// Otherwise ensure that the guids provided exist.
fn get_objects(api: &ExternalEditorApi, guids: Guids, mode: Mode) -> Result<Objects> {
    let save = Save::read(api)?;
    match guids.guids {
        Some(guids) => validate_guids(save, guids),
        None => select_objects(save, mode.msg(), guids.all),
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
    // Warning if tag an lua script or xml ui are mismatched
    for object in save.objects.iter() {
        if let (None, false) = (object.valid_lua()?, object.lua_script.is_empty()) {
            warn!("{} has a lua script but no valid lua tag", object);
            #[rustfmt::skip]
            warn!("If you manually removed the tag, use the detach command to remove the lua script");
        }
        if let (None, false) = (object.valid_xml()?, object.xml_ui.is_empty()) {
            warn!("{} has a xml ui but no valid xml tag", object);
            #[rustfmt::skip]
            warn!("If you manually removed the tag, use the detach command to remove the xml ui");
        }
    }

    // Overwrite the save file with the modified objects
    save.write(api)?;

    // Add global lua_script and xml_ui to save
    let mut objects = save.objects.to_values();
    objects.push(serde_json::json!({
        "guid": "-1",
        "script": save.lua_script,
        "ui": save.xml_ui,
    }));

    // Reload save
    api.reload(serde_json::json!(objects))?;
    info!("reloaded {}", save.name.blue());
    Ok(())
}

/// Set the lua script of the save to either `Global.lua` or `Global.ttslua`, if one of them exists in the `path` directory.
/// Set the xml ui of the save to `Global.xml`, if it exists in the `path` directory.
///
/// If the file is empty, this function will use a placeholder text to avoid writing an empty string.
/// See [`Save::write`].
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
