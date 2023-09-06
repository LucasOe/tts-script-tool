use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use log::*;
use notify_debouncer_mini::{self as debouncer, notify::RecursiveMode};
use tts_external_api::messages::Answer;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

/// Show print, log and error messages in the console.
/// If `--watch` mode is enabled, files in that directory will we watched and reloaded on change.
pub fn start(api: ExternalEditorApi, path: Option<PathBuf>) -> Result<()> {
    let console_handle = console(api);
    let watch_handle = path.map(watch);

    // Wait for threads to finish. Threads should only finish if they return an error.
    console_handle.join().unwrap()?;
    watch_handle.map(|handle| handle.join().unwrap());
    Err("console loop was aborted".into())
}

/// Spawns a new thread that listens to the print, log and error messages in the console.
/// All messages get forwarded to port 39997 so that they can be used again.
fn console(api: ExternalEditorApi) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        loop {
            let buffer = api.read_string();
            match serde_json::from_str(&buffer)? {
                Answer::AnswerPrint(answer) => info!("{}", answer.message),
                Answer::AnswerError(answer) => error!("{}", answer.error_message_prefix),
                // When calling `crate::app::reload` in the watch thread,
                // reloading and writing to the save file is causing multiple prints.
                // Answer::AnswerReload(answer) => info!("reloaded {}", answer.save_path),
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
fn watch(path: PathBuf) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        // Constructs a new `ExternalEditorApi` listening to port 39997
        let api = tts_external_api::ExternalEditorApi {
            listener: TcpListener::bind("127.0.0.1:39997")?,
        };

        // Create notify watcher
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = debouncer::new_debouncer(Duration::from_millis(500), None, tx)?;
        debouncer.watcher().watch(&path, RecursiveMode::Recursive)?;

        loop {
            if let Ok(events) = rx.recv().unwrap() {
                let event = events
                    .into_iter()
                    .find(|event| event.kind == debouncer::DebouncedEventKind::Any);

                if let Some(event) = event {
                    // Make `event.path` relative
                    let path = PathBuf::from("./").join(
                        event
                            .path
                            .strip_prefix(std::env::current_dir().unwrap())
                            .unwrap(),
                    );
                    crate::app::reload(&api, path)?;
                }
            }
        }
    })
}
