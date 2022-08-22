use serde_json::{json, Value};
use snailquote::unescape;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use walkdir::WalkDir;

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

// Iterate over dir (non-recursive) and add tag for every file.
fn read_path(path: &str, guid: &str) {
    let path = Path::new(path);
    if !path.exists() {
        return println!("Path doesn't exist");
    }

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let file_name = String::from(entry.file_name().to_string_lossy());
        println!("Adding \"scripts/{}\" as a tag for \"{}\"", file_name, guid);
        add_tag(&file_name, guid);
    }
}

// Add the file as a tag. Tags use scripts/<File>.ttslua as a naming convention.
// Guid has to be global so objects without scripts can execute code.
fn add_tag(file_name: &str, guid: &str) {
    execute_lua_code(
        &format!(
            r#"
                getObjectFromGUID("{guid}").setTags({{"scripts/{file_name}"}})
            "#,
        ),
        "-1",
    );
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
                println!("{}: {}", guid, tags);
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
    let return_value = &unescape(&result["returnValue"].to_string()).unwrap();
    serde_json::from_str(return_value).unwrap()
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
