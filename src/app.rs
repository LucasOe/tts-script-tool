use crate::error::{Error, Result};
use colorize::AnsiColor;
use inquire::Select;
use regex::Regex;
use snailquote::unescape;
use std::fs;
use std::path::{Path, PathBuf};
use tts_external_api::{json, ExternalEditorApi, Value};

/// Attaches the script to an object by adding the script tag and the script,
/// and then reloads the save, the same way it does when pressing "Save & Play".
pub fn attach(api: &ExternalEditorApi, path: &PathBuf, guid: Option<String>) -> Result<()> {
    let path = Path::new(path);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let guid = match guid {
        Some(guid) => guid,
        None => select_object(api)?,
    };
    guid_exists(api, &guid)?;
    let tag = set_tag(api, file_name, &guid)?;
    println!(
        "{} \"{tag}\" as a tag for \"{guid}\"",
        "added:".yellow().bold()
    );
    let file_content = fs::read_to_string(path)?;
    set_script(api, &guid, &file_content, &tag)?;
    api.reload(json!([]))?;
    println!("{}", "reloaded save!".green().bold());
    set_tag(api, file_name, &guid)?;
    println!("To save the applied tag it is recommended to save the game before reloading.");
    Ok(())
}

/// Update the lua scripts and reload the save file.
pub fn reload(api: &ExternalEditorApi, path: &PathBuf) -> Result<()> {
    // map tags to guids
    let script = format!(
        r#"
            list = {{}}
            for _, obj in pairs(getAllObjects()) do
                if obj.hasAnyTag() then
                    list[obj.guid] = obj.getTags()
                end
            end
            return JSON.encode(list)
        "#,
    );

    let guid_tags = api.execute(script)?.return_value;

    // update scripts with setLuaScript(), so objects without a script get updated.
    if let Value::Object(guid_tags) = guid_tags {
        for (guid, tags) in guid_tags {
            if let Value::Array(tags) = tags {
                let (tags, _) = get_valid_tags(tags);
                // ensure that the object only has one valid tag
                let valid_tag: Option<String> = match tags.len() {
                    1 => Some(unescape_value(&tags[0])),
                    0 => None,
                    _ => return Err(Error::ValidTags { guid, tags }),
                };

                if let Some(tag) = valid_tag {
                    let file_path = get_file_from_tag(path, &tag);
                    let file_content = fs::read_to_string(file_path)?;
                    set_script(api, &guid, &file_content, &tag)?;
                }
            }
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

    let message = json!([{
        "guid": "-1",
        "script": global_script,
        "ui": global_ui
    }]);
    api.reload(message)?;
    println!("{}", "reloaded save!".green().bold());

    Ok(())
}

/// Backup current save as file
pub fn backup(api: &ExternalEditorApi, path: &PathBuf) -> Result<()> {
    let mut path = PathBuf::from(path);
    path.set_extension("json");
    let save_path = api.get_scripts()?.save_path;
    fs::copy(&save_path, &path)?;
    println!(
        "{} \"{save_name}\" as \"{path}\"",
        "save:".yellow().bold(),
        save_name = Path::new(&save_path).file_name().unwrap().to_str().unwrap(),
        path = path.to_str().unwrap()
    );
    Ok(())
}

/// Shows the user a list of all objects in the save to select from.
fn select_object(api: &ExternalEditorApi) -> Result<String> {
    let objects = get_objects(api)?;
    let selection = Select::new("Select the object to attach the script to:", objects).prompt()?;
    Ok(unescape_value(&selection))
}

/// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn set_tag(api: &ExternalEditorApi, file_name: &str, guid: &str) -> Result<String> {
    // get existing tags for object
    let tag = format!("scripts/{file_name}");
    let script = format!(
        r#"
            return JSON.encode(getObjectFromGUID("{guid}").getTags())
        "#,
    );

    let return_value = api.execute(script)?.return_value;
    let tags = return_value.as_array().unwrap();

    // set new tags for object
    let (_, mut tags) = get_valid_tags(tags.clone());
    tags.push(Value::String(String::from(&tag)));
    let script = format!(
        r#"
            tags = JSON.decode("{tags}")
            getObjectFromGUID("{guid}").setTags(tags)
        "#,
        tags = json!(tags).to_string().escape_default(),
    );
    api.execute(script)?;

    Ok(tag)
}

/// Sets the script for the object.
fn set_script(api: &ExternalEditorApi, guid: &str, script: &str, tag: &str) -> Result<()> {
    // add lua script for object
    let script = format!(
        r#"
            return JSON.encode(getObjectFromGUID("{guid}").setLuaScript("{}"))
        "#,
        script.escape_default()
    );
    api.execute(script)?;

    println!("{} {guid} with tag {tag}", "updated:".yellow().bold());
    Ok(())
}

/// Returns a list of all guids
pub fn get_objects(api: &ExternalEditorApi) -> Result<Vec<Value>> {
    let script = format!(
        r#"
            list = {{}}
            for _, obj in pairs(getAllObjects()) do
                table.insert(list, obj.guid)
            end
            return JSON.encode(list)
        "#,
    );
    let objects = api.execute(script)?.return_value;
    Ok(objects.as_array().unwrap().to_owned())
}

/// Split the tags into valid and non valid tags
// Get the tags that follow the "scripts/<File>.ttslua" naming convention.
fn get_valid_tags(tags: Vec<Value>) -> (Vec<Value>, Vec<Value>) {
    let exprs = Regex::new(r"^(scripts/)[\d\w]+(\.lua|.ttslua)$").unwrap();
    let (valid, invalid): (Vec<Value>, Vec<Value>) = tags
        .into_iter()
        .partition(|tag| exprs.is_match(&unescape_value(tag)));

    (valid, invalid)
}

/// Gets the corresponding from the path according to the tag. Path has to be a directory.
fn get_file_from_tag(path: &PathBuf, tag: &str) -> String {
    let path = Path::new(path);
    let file_name = Path::new(&tag).file_name().unwrap();
    let file_path = path.join(file_name);
    String::from(file_path.to_string_lossy())
}

/// Returns an Error if the guid doesn't exist in the current save
fn guid_exists(api: &ExternalEditorApi, guid: &String) -> Result<()> {
    let objects = get_objects(api)?;
    if !objects.contains(&json!(guid)) {
        return Err(Error::MissingGuid(guid.to_owned()));
    }
    Ok(())
}

/// Unescape a Value and returns it as a String.
fn unescape_value(value: &Value) -> String {
    unescape(&value.to_string()).unwrap()
}
