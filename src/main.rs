use serde_json::{json, Value};
use snailquote::unescape;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[allow(unused_variables)]
fn main() {
    let args: Vec<String> = env::args().collect();
    //let url = &args[1];

    // get all guids and the associated tags
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
