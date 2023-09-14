use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use colored::*;
use debounce::EventDebouncer;
use log::*;
use notify::{self, RecursiveMode};
use notify_debouncer_mini::{self as debouncer};
use tts_external_api::messages::Answer;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

use crate::{app, ReloadArgs};

/// Show print, log and error messages in the console.
/// If `--watch` mode is enabled, files in that directory will we watched and reloaded on change.
pub fn start(api: ExternalEditorApi, path: Option<Vec<PathBuf>>) -> Result<()> {
    let console_handle = console(api, path.is_some());
    let watch_handle = path.map(watch);

    // Wait for threads to finish. Threads should only finish if they return an error.
    console_handle.join().unwrap()?;
    watch_handle.map(|handle| handle.join().unwrap());
    Err("console loop was aborted".into())
}

struct ComparableAnswer(Answer);

impl PartialEq for ComparableAnswer {
    fn eq(&self, other: &Self) -> bool {
        // Compare the Enum variant
        std::mem::discriminant(&self.0) == std::mem::discriminant(&other.0)
    }
}

/// Spawns a new thread that listens to the print, log and error messages in the console.
/// All messages get forwarded to port 39997 so that they can be used again.
fn console(api: ExternalEditorApi, watching: bool) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        loop {
            // Forward the message to the TcpStream on port 39997 if a connection exists
            let buffer = api.read_string();
            let message: Answer = serde_json::from_str(&buffer)?;

            // Note: When reloading there isn't a strict order of messages sent from the server
            match message {
                // Only forward `Answer::AnswerReload` messages if watching
                Answer::AnswerReload(_) if watching => {
                    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:39997") {
                        stream.write_all(buffer.as_bytes())?;
                        stream.flush()?;
                    }
                }

                // Print all messages
                //
                // Skips `Answer::AnswerReload` when watching, otherwise reloading
                // would cause multiple messages to print
                _ => {
                    // I'm not sure why, but the order of messages is more consistent with a debouncer
                    let debouncer = EventDebouncer::new(
                        Duration::from_millis(10),
                        move |data: ComparableAnswer| {
                            if let Some(msg) = data.0.message() {
                                let time = chrono::Local::now().format("%H:%M:%S").to_string();
                                println!("[{}] {}", time.bright_white(), msg);
                            };
                        },
                    );
                    debouncer.put(ComparableAnswer(message));
                }
            }
        }
    })
}

trait Message {
    fn message(&self) -> Option<ColoredString>;
}

impl Message for Answer {
    fn message(&self) -> Option<ColoredString> {
        match self {
            Answer::AnswerPrint(answer) => Some(answer.message.bright_white()),
            Answer::AnswerError(answer) => Some(answer.error_message_prefix.red()),
            Answer::AnswerReload(_) => Some("Loading complete.".green()),
            _ => None,
        }
    }
}

/// Spawns a new thread that listens to file changes in the `watch` directory.
/// This thread uses its own `ExternalEditorApi` listening to port 39997.
fn watch(paths: Vec<PathBuf>) -> JoinHandle<Result<()>> {
    thread::spawn(move || -> Result<()> {
        // Constructs a new `ExternalEditorApi` listening to port 39997
        let api = tts_external_api::ExternalEditorApi {
            listener: TcpListener::bind("127.0.0.1:39997")?,
        };

        // Create notify watcher
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = debouncer::new_debouncer(Duration::from_millis(500), tx)?;

        for path in &paths {
            watcher.watcher().watch(path, RecursiveMode::Recursive)?;
        }

        while let Ok(result) = rx.recv() {
            match result {
                Ok(events) => {
                    let paths = events
                        .into_iter()
                        .filter(|event| event.kind == debouncer::DebouncedEventKind::Any)
                        .filter_map(|event| event.path.strip_current_dir().ok())
                        .collect();

                    app::reload(&api, paths, ReloadArgs { guid: None })?
                }
                Err(err) => error!("{}", err),
            }
        }

        Ok(())
    })
}

trait StripCurrentDir {
    fn strip_current_dir(&self) -> Result<PathBuf>;
}

impl StripCurrentDir for PathBuf {
    fn strip_current_dir(&self) -> Result<PathBuf> {
        let path = self.strip_prefix(std::env::current_dir()?)?;
        Ok(PathBuf::from(".\\").join(path))
    }
}
