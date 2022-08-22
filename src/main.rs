use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colorize::AnsiColor;
use regex::Regex;
use serde_json::{json, Map, Value};
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
        Commands::Attach { path, guid } => {
            let file_name = get_file_name(path)?;
            match guid {
                Some(guid) => set_tag(file_name, guid)?,
                None => todo!("List objects to select from"),
            }
        }
        Commands::Reload { path } => {
            reload(path)?;
        }
    }
    Ok(())
}

// Verify valid path and set tag for object with guid.
fn get_file_name(path: &PathBuf) -> Result<&str> {
    let path = Path::new(path);
    if path.exists() && path.is_file() {
        let file_name = path.file_name().unwrap();
        Ok(file_name.to_str().unwrap())
    } else {
        bail!("{:?} is not a file", path)
    }
}

// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn set_tag(file_name: &str, guid: &str) -> Result<()> {
    println!("Adding \"scripts/{}\" as a tag for \"{}\"", file_name, guid);
    execute_lua_code(
        &format!(
            r#"
                getObjectFromGUID("{guid}").setTags({{"scripts/{file_name}"}})
            "#,
        ),
        "-1",
    )?;
    Ok(())
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
            _ => bail!("{} has multiple valid tags", guid),
        }
    } else {
        Ok(None)
    }
}

// Update the lua scripts and reload the save file.
fn reload(path: &PathBuf) -> Result<()> {
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
        "-1",
    )?;
    let mut script_list: Map<String, Value> = Map::new();
    if let Value::Object(guid_tags) = guid_tags {
        // get scripts from tags and store them in script_list
        for (guid, tags) in guid_tags {
            if let Some(tag) = get_valid_tags(tags, &guid)? {
                let file_path = get_file_from_tag(path, &tag, &guid)?;
                let file_content = fs::read_to_string(file_path)?;
                println!(
                    "{} {} with tag {:?}",
                    format!("updating:").green().bold(),
                    guid,
                    tag
                );
                script_list.insert(guid.clone(), Value::String(file_content));
            }
        }
    }
    let save_data = get_lua_scripts()?;
    let script_states = &save_data["scriptStates"];
    if let Value::Array(objects) = script_states {
        for object in objects {
            if let Value::Object(object) = object {
                if let Value::String(guid) = object.get("guid").unwrap() {
                    let local_script = script_list.get(guid);
                    match local_script {
                        Some(local_script) => {
                            println!("{}: {}\n", guid, local_script);
                            // Todo: Update scriptStates and reload
                        }
                        None => continue,
                    }
                }
            }
        }
    }
    Ok(())
}

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

// Executes lua code inside Tabletop Simulator and returns the value.
// Pass a guid of "-1" to execute code globally. When using the print
// function inside the code, the return value may not get passed correctly!
// Returns Null if the code returns nothing.
fn execute_lua_code(code: &str, guid: &str) -> Result<Value> {
    let data = send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": guid,
            "script": code
        })
        .to_string(),
    )?;
    let result: Value = serde_json::from_str(&data).unwrap();
    let return_value = unescape_value(&result["returnValue"]);
    Ok(serde_json::from_str(&return_value).unwrap())
}

// Get lua scripts
#[allow(dead_code)]
fn get_lua_scripts() -> Result<Value> {
    let data = send(
        json!({
            "messageID": 0,
        })
        .to_string(),
    )?;
    Ok(serde_json::from_str(&data).unwrap())
}

fn unescape_value(value: &Value) -> String {
    unescape(&value.to_string()).unwrap()
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
