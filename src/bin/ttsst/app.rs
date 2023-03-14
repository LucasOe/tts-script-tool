use inquire::Select;
use std::fs;
use std::path::{Path, PathBuf};
use tts_external_api::ExternalEditorApi;
use ttsst::error::{Error, Result};
use ttsst::objects::{Object, Objects};
use ttsst::tags::Tag;
use ttsst::{print_info, reload};

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: &Path, guid: Option<String>) -> Result<()> {
    let object = get_object(api, guid)?;

    let tag = Tag::from(path);
    set_tag(api, &object, &tag)?;
    print_info!("added:", "'{tag}' as a tag to {object}");

    let script = fs::read_to_string(path)?;
    object.set_script(api, script)?;
    print_info!("updated:", "{object} with tag '{tag}'");

    reload!(api, [])?;
    set_tag(api, &object, &tag)?;

    print_info!("reloaded save!");
    println!("To save the applied tag it is recommended to save the game before reloading.");
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: &Path) -> Result<()> {
    for object in Objects::request(api)? {
        let tag = object.tags(api)?;
        // Update the script with the file content from the tag
        if let Some(tag) = tag.valid()? {
            let script = tag.read_file(path)?;
            object.set_script(api, script)?;
            print_info!("updated:", "{object} with tag '{tag}'");
        }
    }

    // Get global script and ui and reload the current save.
    // Gets the global script and ui from files on the provided path first.
    // If no files exist, it uses the script and the ui from the current save.
    let script_states = Objects::request_script_states(api)?;
    let script_state = &script_states.global().unwrap();
    reload!(
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
/// Otherwise ensure that the guid provided exists.
fn get_object(api: &ExternalEditorApi, guid: Option<String>) -> Result<Object> {
    match guid {
        Some(guid) => Object::new(guid).exists(api),
        None => select_object(api),
    }
}

/// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
fn set_tag(api: &ExternalEditorApi, object: &Object, tag: &Tag) -> Result<()> {
    // Set new script tag for object and add the previous invalid tags.
    // Previous valid tags will be overwritten with the new script tag.
    let mut tags = object.tags(api)?.filter_invalid();
    tags.push(tag.clone());
    object.set_tags(api, &tags)?;
    Ok(())
}

/// Get a global script from a file or get the script from the current save if no file exists.
/// Returns [`Error::Msg`] if both "Global.ttslua" and "Global.lua" exist.
/// If the file exists but can't be read it returns [`Error::Io`].
fn get_global_script(path: &Path, script_state: &Object) -> Result<String> {
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
fn get_global_ui(path: &Path, script_state: &Object) -> Result<String> {
    let global_xml = Path::new(path).join("./Global.xml");
    match global_xml.exists() {
        true => fs::read_to_string(global_xml).map_err(|_| Error::ReadFile),
        false => Ok(script_state.ui()),
    }
}

/// Show a prompt for selecting an object from the current save
fn select_object(api: &ExternalEditorApi) -> Result<Object> {
    let objects = Objects::request(api)?.into_inner();
    Select::new("Select the object to attach the script to:", objects)
        .prompt()
        .map_err(Error::InquireError)
}
