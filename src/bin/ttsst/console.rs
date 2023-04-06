use colorize::AnsiColor;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use tts_external_api::messages::Answer;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

/// Spawns a new thread that listens to the print, log and error messages in the console.
/// All messages get forwarded to port 39997 so that they can be used again.
pub fn console(api: ExternalEditorApi) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        loop {
            let buffer = api.read_string();
            match serde_json::from_str(&buffer)? {
                Answer::AnswerPrint(answer) => println!("{}", answer.message.b_grey()),
                Answer::AnswerReload(_answer) => println!("{}", "Loading complete.".green()),
                Answer::AnswerError(answer) => println!("{}", answer.error_message_prefix.red()),
                _ => {}
            }

            // Forward the message to the TcpStream on port 39997 if a connection exists
            if let Ok(mut stream) = TcpStream::connect("127.0.0.1:39997") {
                stream.write_all(buffer.as_bytes())?;
                stream.flush()?;
            }
        }
    })
}

/// Spawns a new thread that listens to file changes in the `watch` directory.
/// This thread uses its own `ExternalEditorApi` listening to port 39997.
pub fn watch() -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        // Constructs a new `ExternalEditorApi` listening to port 39997
        let api = tts_external_api::ExternalEditorApi {
            listener: TcpListener::bind("127.0.0.1:39997")?,
        };

        loop {
            thread::sleep(std::time::Duration::from_secs(5));
            api.execute(r#"print("Hello")"#.to_string())?;
        }
    })
}
