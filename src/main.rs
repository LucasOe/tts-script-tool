use serde_json::json;
use std::io::Write;
use std::net::TcpStream;

fn main() {
    // first
    let stream = TcpStream::connect("127.0.0.1:39999").unwrap();
    write(
        &stream,
        json!({
            "messageID": 3,
            "guid":"-1",
            "script":"print(\"Hello, World\")"
        })
        .to_string(),
    );

    // second
    let new_stream = TcpStream::connect("127.0.0.1:39999").unwrap();
    write(
        &new_stream,
        json!({
            "messageID": 3,
            "guid":"-1",
            "script":"print(\"Hello, Mars\")"
        })
        .to_string(),
    );
}

fn write(mut stream: &TcpStream, msg: String) {
    stream.write_all(msg.as_bytes()).unwrap();
    stream.flush().unwrap();
}
