use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use colored::*;
use itertools::Itertools;
use log::*;
use notify::{self, RecursiveMode};
use notify_debouncer_mini::{self as debouncer};
use serde_json::json;
use tts_external_api::messages::{Answer, MessageReload};
use tts_external_api::ExternalEditorApi;
use ttsst::save::SaveFile;

use crate::app::SaveFileExt;
use crate::ReloadArgs;

/// Show print, log and error messages in the console.
/// If `--watch` mode is enabled, files in that directory will we watched and reloaded on change.
pub fn start<P: AsRef<Path> + Clone + Sync>(api: &ExternalEditorApi, paths: Option<&[P]>) {
    // Note: `std::process::exit` terminates all running threads
    std::thread::scope(|scope| {
        scope.spawn(move || {
            if let Err(err) = console(api, paths) {
                error!("{}", err);
                std::process::exit(1);
            }
        });

        if let Some(paths) = paths {
            scope.spawn(move || {
                if let Err(err) = watch(api, paths) {
                    error!("{}", err);
                    std::process::exit(1);
                }
            });
        }
    });
}

/// Spawns a new thread that listens to the print, log and error messages in the console.
/// All messages get forwarded to port 39997 so that they can be used again.
fn console<P: AsRef<Path> + Clone>(
    api: &ExternalEditorApi,
    paths: Option<&[P]>,
) -> Result<Infallible> {
    loop {
        let message = api.read();

        // Reload changes if the save gets reloaded while in watch mode
        if let (Answer::AnswerReload(answer), Some(paths)) = (&message, &paths) {
            // Clear screen and put the cursor at the first row and first column of the screen.
            print!("\x1B[2J\x1B[1;1H");

            let mut save = SaveFile::read_from_path(&answer.save_path)?;
            save.reload(api, paths, ReloadArgs { guid: None })?;
        }

        // Print all messages
        //
        // Skips `Answer::AnswerReload` when watching, otherwise reloading
        // would cause multiple messages to print
        if let Some(msg) = message.message() {
            let time = chrono::Local::now().format("%H:%M:%S").to_string();
            println!("[{}] {}", time.bright_white(), msg);
        }
    }
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
fn watch<P: AsRef<Path>>(api: &ExternalEditorApi, paths: &[P]) -> Result<Infallible> {
    // Create notify watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = debouncer::new_debouncer(Duration::from_millis(500), tx)?;

    for path in paths {
        watcher
            .watcher()
            .watch(path.as_ref(), RecursiveMode::Recursive)?;
    }

    loop {
        match rx.recv()? {
            Ok(events) => {
                let paths = events
                    .iter()
                    .filter(|event| event.kind == debouncer::DebouncedEventKind::Any)
                    .filter_map(|event| event.path.strip_current_dir().ok())
                    .collect_vec();

                if !paths.is_empty() {
                    // Send ReloadMessage. Waiting for an answer would block the thread,
                    // because the tcp listener is already in use.
                    api.send(MessageReload::new(json!([])).as_message())?;
                }
            }
            Err(err) => error!("{}", err),
        }
    }
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
