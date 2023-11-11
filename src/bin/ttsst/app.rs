use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;
use derive_more::Display;
use itertools::Itertools;
use log::*;
use path_slash::PathExt;
use tts_external_api::ExternalEditorApi;
use ttsst::{Object, Objects, Save, Tag};

use crate::{Guids, ReloadArgs};

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
pub fn attach<P: AsRef<Path>>(api: &ExternalEditorApi, path: P, guids: Guids) -> Result<()> {
    let mut objects = get_objects(api, guids, Mode::Attach)?;

    let tag = Tag::try_from(path.as_ref())?;
    let file = read_file(path)?;
    for object in objects.iter_mut() {
        // Add lua tag to objects
        if tag.is_lua() {
            object.tags.retain(|tag| !tag.is_lua());
            object.tags.push(tag.clone());
            object.lua_script = file.clone();
            info!("attached script to {object}");
        }
        // Add xml tag to objects
        if tag.is_xml() {
            object.tags.retain(|tag| !tag.is_xml());
            object.tags.push(tag.clone());
            object.xml_ui = file.clone();
            info!("attached ui element to {object}");
        }
    }

    // Add objects to a new save state
    let mut save = Save::read(api)?;
    save.objects.replace(&mut objects);

    update_save(api, &mut save)?;
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

    update_save(api, &mut save)?;
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload<P: AsRef<Path> + Clone>(
    api: &ExternalEditorApi,
    paths: &[P],
    args: ReloadArgs,
) -> Result<()> {
    let mut save = Save::read(api)?;

    let mut has_changed = false;
    for path in &paths.reduce::<Vec<_>>() {
        // Reload objects
        if let Some(guid) = &args.guid {
            let object = save.objects.find_object_mut(guid)?;
            has_changed = reload_object(object, path)?;
        } else {
            for object in save.objects.iter_mut() {
                has_changed = reload_object(object, path)?;
            }
        }

        // Add the paths as a component tag, so that in watch mode reloaded paths
        // will show up as tags.
        // NOTE: These have to be cleaned up after exiting watch mode
        if let Ok(tag) = Tag::try_from(path.as_ref()) {
            has_changed = save.push_object_tag(tag);
        }
    }

    if has_changed {
        update_global_files(&mut save, paths)?;
        update_save(api, &mut save)?;
    }
    Ok(())
}

/// Reload the lua script and xml ui of an `object`, if its tag matches the `path`
pub fn reload_object<P: AsRef<Path>>(object: &mut Object, path: P) -> Result<bool> {
    // Update lua scripts if the path is a lua file
    let lua_change = match object.valid_lua()? {
        Some(tag) if tag.starts_with(&path) => {
            let file = read_file(tag.path()?)?;
            if object.lua_script != file {
                object.lua_script = file;
                info!("updated {object}");
                true
            } else {
                false
            }
        }
        // Remove lua script if the objects has no valid tag
        None if !object.lua_script.is_empty() => {
            object.lua_script = "".to_string();
            info!("removed lua script from {}", object);
            true
        }
        _ => false,
    };
    // Update xml ui if the path is a xml file
    let xml_change = match object.valid_xml()? {
        Some(tag) if tag.starts_with(&path) => {
            let file = read_file(tag.path()?)?;
            if object.xml_ui != file {
                object.xml_ui = read_file(tag.path()?)?;
                info!("updated {object}");
                true
            } else {
                false
            }
        }
        // Remove xml ui if the objects has no valid tag
        None if !object.xml_ui.is_empty() => {
            object.xml_ui = "".to_string();
            info!("removed xml ui from {}", object);
            true
        }
        _ => false,
    };

    Ok(lua_change || xml_change)
}

/// Backup current save as file
pub fn backup<P: AsRef<Path>>(api: &ExternalEditorApi, path: P) -> Result<()> {
    let save_path = api.get_scripts()?.save_path;
    fs::copy(&save_path, &path)?;

    // Print information about the file
    let save_name = Path::new(&save_path).file_name().unwrap().to_str().unwrap();
    let path_display = path.as_ref().to_slash_lossy();
    #[rustfmt::skip]
    info!("save '{}' as '{}'", save_name.yellow(), path_display.yellow());

    Ok(())
}

/// If no guids are provided show a selection of objects in the current savestate.
/// Otherwise ensure that the guids provided exist.
fn get_objects(api: &ExternalEditorApi, guids: Guids, mode: Mode) -> Result<Objects> {
    let save = Save::read(api)?;
    match guids.guids {
        Some(guids) => find_objects(save, &guids),
        None => select_objects(save, mode.msg(), guids.all),
    }
}

/// Once an `Result::Err` is found, the iteration will terminate and return the result.
/// If `guids` only contains existing objects, a vec with the savestate of those objects will be returned.
fn find_objects<T: AsRef<str>>(save: Save, guids: &[T]) -> Result<Objects> {
    guids
        .as_ref()
        .iter()
        .map(|guid| {
            save.objects
                .find_object(guid)
                .map_err(|err| err.into())
                .cloned()
        })
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
pub fn update_save(api: &ExternalEditorApi, save: &mut Save) -> Result<()> {
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

    // Remove component tags, if they exist as object tags
    save.remove_object_tags();

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
pub fn update_global_files<P: AsRef<Path>>(save: &mut Save, paths: &[P]) -> Result<()> {
    const GLOBAL_LUA: &[&str] = &["Global.lua", "Global.ttslua"];
    const GLOBAL_XML: &[&str] = &["Global.xml"];

    // Filter out duplicates
    let unique_paths = paths
        .iter()
        .unique_by(|path| path.as_ref().to_path_buf())
        .collect_vec();

    if let Some(path) = get_global_path(&unique_paths, GLOBAL_LUA)? {
        let file = read_file(&path)?;
        let lua_script = match file.is_empty() {
            #[rustfmt::skip]
            true => "--[[ Lua code. See documentation: https://api.tabletopsimulator.com/ --]]".to_string(),
            false => file,
        };
        if save.lua_script != lua_script {
            #[rustfmt::skip]
            info!("updated {} using '{}'", "Global Lua".yellow(), path.to_slash_lossy().yellow());
            save.lua_script = lua_script;
        };
    };

    // Update xml_ui
    if let Some(path) = get_global_path(&unique_paths, GLOBAL_XML)? {
        let file: String = read_file(&path)?;
        let xml_ui = match file.is_empty() {
            #[rustfmt::skip]
            true => "<!-- Xml UI. See documentation: https://api.tabletopsimulator.com/ui/introUI/ -->".to_string(),
            false => file,
        };
        if save.xml_ui != xml_ui {
            #[rustfmt::skip]
            info!("updated {} using '{}'", "Global UI".yellow(), path.to_slash_lossy().yellow());
            save.xml_ui = xml_ui;
        };
    };

    Ok(())
}

/// Returns a path to a global script, by joining `paths` and `files`.
fn get_global_path<P: AsRef<Path>, T: AsRef<str>>(
    paths: &[P],
    files: &[T],
) -> Result<Option<PathBuf>> {
    // Returns a list of joined `paths` and `files` that exist
    let joined_paths = paths
        .iter()
        .flat_map(|path| {
            files
                .iter()
                .filter_map(|file| match path.as_ref().is_dir() {
                    // If path is a dir, join `file`
                    true => Some(path.as_ref().join(file.as_ref())),
                    // If path ends with `file`, it is a global file
                    false if path.as_ref().file_name() == Some(OsStr::new(file.as_ref())) => {
                        Some(path.as_ref().to_path_buf())
                    }
                    // if path is a file that doesn't end with `file`, ignore it
                    false => None,
                })
                .filter(|path| path.exists())
                .collect_vec()
        })
        .collect_vec();

    match joined_paths.len() {
        0 | 1 => Ok(joined_paths.get(0).map(ToOwned::to_owned)),
        _ => select_paths(&joined_paths).map(Option::Some),
    }
}

/// Shows a multi selection prompt of `paths`
fn select_paths<P: AsRef<Path>>(paths: &[P]) -> Result<PathBuf> {
    #[derive(Display)]
    #[display(fmt = "'{}'", "self.0.as_ref().to_slash_lossy().yellow()")]
    struct DisplayPath<P: AsRef<Path>>(P);

    // Wrap `paths` in `DisplayPath` so they can be displayed by the inquire prompt
    let display_paths = paths.iter().map(DisplayPath).collect_vec();

    match inquire::Select::new("Select a Global file to use:", display_paths).prompt() {
        Ok(path) => Ok(path.0.as_ref().to_path_buf()),
        Err(err) => Err(err.into()),
    }
}

trait Reduce<P> {
    /// Filters and deduplicates the collection of paths, returning a new collection.
    ///
    /// This method removes duplicate paths based on their logical content and ensures that
    /// subfolders are not included if a parent folder is present in the collection.
    fn reduce<T: FromIterator<P>>(&self) -> T;
}

impl<U: AsRef<[P]>, P: AsRef<Path> + Clone> Reduce<P> for U {
    fn reduce<T: FromIterator<P>>(&self) -> T {
        self.as_ref()
            .iter()
            .unique_by(|path| path.as_ref().to_path_buf())
            .filter(|&this| {
                !self.as_ref().iter().any(|other| {
                    let paths = (this.as_ref(), other.as_ref());
                    paths.0 != paths.1 && paths.0.starts_with(paths.1)
                })
            })
            .cloned()
            .collect()
    }
}

/// Reads a file from the path and replaces every occurrence of `\t` with spaces.
fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content.replace('\t', "    ")),
        Err(err) => Err(err.into()),
    }
}
