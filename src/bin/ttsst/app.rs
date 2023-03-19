use std::fs;
use std::path::{Path, PathBuf};

use crate::print_info;
use inquire::MultiSelect;
use tts_external_api::ExternalEditorApi;
use ttsst::error::{Error, Result};
use ttsst::reload;
use ttsst::save::{Object, Save};
use ttsst::tags::Tag;

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: &Path, guids: Option<Vec<String>>) -> Result<()> {
    let mut objects = get_objects(api, guids)?;

    let tag = Tag::from(path);
    let script = fs::read_to_string(path)?.replace("\t", "    ");
    // Add tag and script to objects
    for mut object in &mut objects {
        let mut new_tags = object.clone().tags.filter_invalid(); // Todo: Remove clone if possible
        new_tags.push(tag.clone());
        object.tags = new_tags;
        print_info!("added:", "'{tag}' as a tag to {object}");

        object.lua_script = script.clone();
        print_info!("added:", "{path:?} as a script to {object}");
    }

    // Overwrite the save file with the modified objects
    Save::read_save(api)?
        .add_objects(objects)?
        .write_save(api)?;

    // Reload new save file
    reload!(api, [])?; // Todo: Reloading does not reload the save file
    print_info!("reloaded save!");

    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(_api: &ExternalEditorApi, _path: &Path) -> Result<()> {
    todo!();
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

/// If no guids are provided show a selection of objects in the current savestate.
/// Otherwise ensure that the guids provided exist.
fn get_objects(api: &ExternalEditorApi, guids: Option<Vec<String>>) -> Result<Vec<Object>> {
    let objects = Save::read_save(api)?.object_states;

    match guids {
        Some(guids) => {
            // Once an `Result::Err` is found, the iteration will terminate and return the result.
            // If `guids` only contains existing objects, a vec with the savestate of those objects will be returned.
            guids
                .into_iter()
                .map(|guid| {
                    objects
                        .iter()
                        .find(|object| object.has_guid(&guid))
                        .cloned()
                        .ok_or::<Error>(format!("{guid} does not exist").into())
                })
                .collect() // `Vec<Result<T, E>>` gets turned into `Result<Vec<T>, E>`
        }
        None => {
            // Shows a multi selection prompt
            MultiSelect::new("Select the object to attach the script to:", objects)
                .prompt()
                .map_err(Error::InquireError)
        }
    }
}
