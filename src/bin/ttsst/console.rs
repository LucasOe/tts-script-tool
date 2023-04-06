use colorize::AnsiColor;
use std::io::Write;
use std::thread::{self, JoinHandle};
use tts_external_api::messages::Answer;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

/// Spawns a new thread that listens to the print, log and error messages in the console
/// All messages get forwarded to port 39997 so that they can be used again
pub fn console(api: ExternalEditorApi) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        loop {
            // Create stream on port 39997 to allow the construction of a second `ExternalEditorApi` without errors
            let mut stream = std::net::TcpStream::connect("127.0.0.1:39997")?;

            let buffer = api.read_string();
            match serde_json::from_str(&buffer)? {
                Answer::AnswerPrint(answer) => {
                    println!("{}", answer.message.b_grey())
                }
                Answer::AnswerReload(_answer) => {
                    println!("{}", "Loading complete.".green())
                }
                Answer::AnswerError(answer) => {
                    println!("{}", answer.error_message_prefix.red())
                }
                _ => {}
            }

            // Forward the message to the TcpStream
            stream.write_all(&buffer.as_bytes())?;
            stream.flush()?;
        }
    })
}

/// Spawns a new thread that listens to file changes in the `watch` directory
pub fn watch(api: ExternalEditorApi) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        // Accept the incoming TcpStream
        api.listener.accept()?;

        loop {
            thread::sleep(std::time::Duration::from_secs(1));
        }
    })
}
