use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colorize::AnsiColor;
use regex::Regex;
use serde_json::{json, Value};
use snailquote::unescape;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Attach script to object
    Attach {
        /// Path to the file that should be attached
        #[clap(parse(from_os_str))]
        path: PathBuf,
        /// Optional: The guid of the object the script should be attached to.
        /// If not provided a list of all objects will be shown.
        #[clap(value_parser)]
        guid: Option<String>,
    },
    /// Update scripts and reload save
    Reload {
        /// Path to the directory with all scripts
        #[clap(parse(from_os_str))]
        path: PathBuf,
    },
}

fn main() {
    let args = Args::parse();

    if let Err(err) = run(args) {
        println!("{} {}", format!("error:").red().bold(), err);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    match &args.command {
        Commands::Attach { path, guid } => match guid {
            Some(guid) => attach(path, guid)?,
            None => todo!("List objects to select from"),
        },
        Commands::Reload { path } => {
            reload(path)?;
        }
    }
    Ok(())
}

// Attaches the script to an object by adding the script tag and the script,
// and then reloading the save.
fn attach(path: &PathBuf, guid: &String) -> Result<()> {
    let path = Path::new(path);
    if path.exists() && path.is_file() {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let tag = set_tag(file_name, &guid)?;
        let file_content = fs::read_to_string(path)?;
        set_script(&guid, &file_content, &tag)?;
        save_and_play(json!([]))?;
    } else {
        bail!("{:?} is not a file", path)
    }
    Ok(())
}

// Update the lua scripts and reload the save file.
fn reload(path: &PathBuf) -> Result<()> {
    // map tags to guids
    let guid_tags = execute_lua_code(
        r#"
            list = {}
            for _, obj in pairs(getAllObjects()) do
                if obj.hasAnyTag() then
                    list[obj.guid] = obj.getTags()
                end
            end
            return JSON.encode(list)
        "#,
    )?;
    // update scripts with setLuaScript(), so objects without a script get updated.
    if let Value::Object(guid_tags) = guid_tags {
        for (guid, tags) in guid_tags {
            match get_valid_tags(tags, &guid) {
                Ok(valid_tags) => {
                    if let Some(tag) = valid_tags {
                        let file_path = get_file_from_tag(path, &tag, &guid)?;
                        let file_content = fs::read_to_string(file_path)?;
                        set_script(&guid, &file_content, &tag)?;
                    }
                }
                Err(err) => println!("{} {}", format!("error:").red().bold(), err),
            }
        }
    }
    // get scriptStates
    let save_data = get_lua_scripts()?;
    let script_states = save_data["scriptStates"].as_array().unwrap();
    // add global script to script_list
    let global_path = Path::new(path).join("./Global.ttslua");

    let message = json!([{
        "guid": "-1",
        "script": match fs::read_to_string(global_path) {
            Ok(global_file_content) => global_file_content,
            Err(_) => unescape(&script_states[0].get("script").unwrap().to_string()).unwrap(),
        },
        "ui": unescape(&script_states[0].get("ui").unwrap().to_string()).unwrap()
    }]);
    save_and_play(message)?;

    Ok(())
}

// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn set_tag(file_name: &str, guid: &str) -> Result<String> {
    let tag = format!("scripts/{file_name}");
    println!(
        "{} \"{tag}\" as a tag for \"{guid}\"",
        format!("added:").yellow().bold()
    );
    execute_lua_code(&format!(
        r#"
            getObjectFromGUID("{guid}").setTags({{"{tag}"}})
        "#,
    ))?;
    Ok(tag)
}

// Sets the script for the object.
fn set_script(guid: &String, script: &String, tag: &str) -> Result<bool> {
    let result = execute_lua_code(&format!(
        r#"
            return getObjectFromGUID("{guid}").setLuaScript("{}")
        "#,
        script.escape_default()
    ))?
    .as_bool()
    .unwrap();
    if result {
        println!(
            "{} {guid} with tag {tag}",
            format!("updated:").yellow().bold()
        );
    }
    Ok(result)
}

// Get the tags that follow the "scripts/<File>.ttslua" naming convention.
// Returns None if there are multiple valid tags.
fn get_valid_tags(tags: Value, guid: &String) -> Result<Option<String>> {
    if let Value::Array(tags) = tags {
        let exprs = Regex::new(r"^(scripts/)[\d\w]+(\.ttslua)$").unwrap();
        let valid_tags: Vec<Value> = tags
            .into_iter()
            .filter(|tag| exprs.is_match(&unescape_value(tag)))
            .collect();

        match valid_tags.len() {
            1 => Ok(Some(unescape_value(&valid_tags[0]))),
            0 => Ok(None),
            _ => bail!("{} has multiple script tags", guid),
        }
    } else {
        Ok(None)
    }
}

// Gets the corresponding from the path according to the tag. Path has to be a directory.
fn get_file_from_tag(path: &PathBuf, tag: &String, guid: &String) -> Result<String> {
    let path = Path::new(path);
    let file_name = Path::new(&tag).file_name().unwrap();
    if path.exists() && path.is_dir() {
        let file_path = path.join(file_name);
        if file_path.exists() && file_path.is_file() {
            Ok(String::from(file_path.to_string_lossy()))
        } else {
            bail!("file for {:?} with tag {} not found", guid, tag)
        }
    } else {
        bail!("{:?} is not a directory", path)
    }
}

// Unescapes a Value and returns it as a String.
fn unescape_value(value: &Value) -> String {
    unescape(&value.to_string()).unwrap()
}

// Get lua scripts
fn get_lua_scripts() -> Result<Value> {
    let data = send(
        json!({
            "messageID": 0,
        })
        .to_string(),
    )?;
    Ok(serde_json::from_str(&data).unwrap())
}

// Update the lua scripts and UI XML for any objects listed in the message,
// and then reload the save file. Objects not mentioned are not updated.
fn save_and_play(script_states: Value) -> Result<()> {
    send(
        json!({
            "messageID": 1,
            "scriptStates": script_states
        })
        .to_string(),
    )?;
    println!("{}", format!("reloaded save!").green().bold());
    Ok(())
}

// Executes lua code inside Tabletop Simulator and returns the value.
// Pass a guid of "-1" to execute code globally. When using the print
// function inside the code, the return value may not get passed correctly!
// Returns Null if the code returns nothing.
fn execute_lua_code(code: &str) -> Result<Value> {
    let data = send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": "-1",
            "script": code
        })
        .to_string(),
    )?;
    let result: Value = serde_json::from_str(&data).unwrap();
    let return_value = unescape_value(&result["returnValue"]);
    Ok(serde_json::from_str(&return_value).unwrap())
}

// Sends a message to Tabletop Simulator and returns the answer as a String.
fn send(msg: String) -> Result<String> {
    let mut stream = TcpStream::connect("127.0.0.1:39999")?;
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();

    let listener = TcpListener::bind("127.0.0.1:39998")?;
    let (stream, _addr) = listener.accept()?;
    Ok(read(&stream))
}

fn read(mut stream: &TcpStream) -> String {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    buffer
}
