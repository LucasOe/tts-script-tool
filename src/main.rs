use std::env;
use std::io::Write;
use std::net::TcpStream;

#[allow(unused_variables)]
fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    let mut stream = TcpStream::connect("127.0.0.1:39999").expect("Couldn't connect to server.");
    println!("Connected!");

    let msg = br#"
        {
            "messageID": 3,
            "guid": "-1",
            "script": "print(\"Hello, World\")"
        }"#;
    stream.write(msg).unwrap();
}
