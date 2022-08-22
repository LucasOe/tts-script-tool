use regex::Regex;
use serde_json::{json, Value};
use snailquote::unescape;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        3 => {
            let command = &args[1];
            match &command[..] {
                "set" => println!("Not enough arguments"),
                "reload" => reload(&args[2]),
                _ => println!("Invalid command"),
            }
        }
        4 => {
            let command = &args[1];
            match &command[..] {
                "set" => read_path(&args[2], &args[3]),
                _ => println!("Invalid command"),
            }
        }
        _ => println!("Invalid arguments"),
    }
}

// Verify valid path and set tag for object with guid.
fn read_path(path: &str, guid: &str) {
    let path = Path::new(path);
    if !path.exists() || path.is_dir() {
        return println!("Path is not a file");
    }

    let file_name = String::from(path.file_name().unwrap().to_string_lossy());
    println!("Adding \"scripts/{}\" as a tag for \"{}\"", file_name, guid);
    set_tag(&file_name, guid);
}

// Add the file as a tag. Tags use "scripts/<File>.ttslua" as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn set_tag(file_name: &str, guid: &str) {
    execute_lua_code(
        &format!(
            r#"
                getObjectFromGUID("{guid}").setTags({{"scripts/{file_name}"}})
            "#,
        ),
        "-1",
    );
}

// Get the tags that follow the "scripts/<File>.ttslua" naming convention.
// Returns None if there are multiple valid tags.
fn get_valid_tags(tags: Value) -> Result<String, &'static str> {
    match tags {
        Value::Array(tags) => {
            let exprs = Regex::new(r"^(scripts/)[\d\w]+(\.ttslua)$").unwrap();
            let valid_tags: Vec<Value> = tags
                .into_iter()
                .filter(|tag| exprs.is_match(&unescape_value(tag)))
                .collect();

            match valid_tags.len() {
                1 => Ok(unescape_value(&valid_tags[0])),
                _ => Err("duplicate tags"),
            }
        }
        _ => Err("not an array"),
    }
}

// Update the lua scripts and reload the save file.
fn reload(_url: &str) {
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
    );
    match guid_tags {
        Value::Object(guid_tags) => {
            for (guid, tags) in guid_tags {
                match get_valid_tags(tags) {
                    Ok(tag) => {
                        println!("{}: {:?}", guid, tag)
                    }
                    Err("duplicate tags") => {
                        println!("Error: {} has multiple valid script tags!", guid)
                    }
                    Err(_) => continue,
                }
            }
        }
        _ => panic!("guid_tags not an object."),
    }
}

// Executes lua code inside Tabletop Simulator and returns the value.
// Pass a guid of "-1" to execute code globally. When using the print
// function inside the code, the return value may not get passed correctly!
// Returns Null if the code returns nothing.
fn execute_lua_code(code: &str, guid: &str) -> Value {
    let data = send(
        json!({
            "messageID": 3,
            "returnID": "5",
            "guid": guid,
            "script": code
        })
        .to_string(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(&data).unwrap();
    let return_value = unescape_value(&result["returnValue"]);
    serde_json::from_str(&return_value).unwrap()
}

fn unescape_value(value: &Value) -> String {
    unescape(&value.to_string()).unwrap()
}

// Sends a message to Tabletop Simulator and returns the answer as a String.
fn send(msg: String) -> Option<String> {
    let mut stream = TcpStream::connect("127.0.0.1:39999").unwrap();
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();

    let listener = TcpListener::bind("127.0.0.1:39998").unwrap();
    match listener.accept() {
        Ok((stream, _addr)) => Some(read(&stream)),
        Err(_) => None,
    }
}

fn read(mut stream: &TcpStream) -> String {
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    stream.flush().unwrap();
    buffer
}
