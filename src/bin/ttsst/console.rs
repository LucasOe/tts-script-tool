use std::convert::Infallible;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use colored::*;
use itertools::Itertools;
use log::*;
use notify::RecursiveMode;
use notify_debouncer_mini::{self as debouncer};
use serde_json::json;
use tts_external_api::messages::{Answer, MessageReload};
use tts_external_api::ExternalEditorApi as Api;
use ttsst::Tag;

use crate::app::SaveFile;
use crate::utils::StripCurrentDir;
use crate::ReloadArgs;

/// Show print, log and error messages in the console.
/// If `--watch` mode is enabled, files in that directory will we watched and reloaded on change.
pub fn start<P>(save_file: &SaveFile, api: &Api, paths: Option<&[P]>)
where
    P: AsRef<Path> + Clone + Sync,
{
    // Note: `std::process::exit` terminates all running threads
    std::thread::scope(|scope| {
        scope.spawn(move || {
            if let Err(err) = read(save_file, api, paths) {
                error!("{}", err);
                std::process::exit(1);
            }
        });

        if let Some(paths) = paths {
            scope.spawn(move || {
                if let Err(err) = watch(save_file, api, paths) {
                    error!("{}", err);
                    std::process::exit(1);
                }
            });
        }
    });
}

/// Spawns a new thread that listens to the print, log and error messages in the console.
/// All messages get forwarded to port 39997 so that they can be used again.
fn read<P>(save_file: &SaveFile, api: &Api, paths: Option<&[P]>) -> Result<Infallible>
where
    P: AsRef<Path> + Clone,
{
    loop {
        let message = api.read();

        // Reload changes if the save gets reloaded while in watch mode
        if let (Answer::AnswerReload(answer), Some(paths)) = (&message, &paths) {
            // Check if the save file of the incoming answer is still the same save file
            let mut answer_save_file = SaveFile::read_from_path(&answer.save_path)?;
            if answer_save_file.path != save_file.path {
                error!("Different save file has been loaded!");
            }

            // Clear screen and put the cursor at the first row and first column of the screen
            print!("\x1B[2J\x1B[1;1H");
            answer_save_file.reload(api, paths, ReloadArgs { guid: None })?;
        }

        // Print messages
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
fn watch<P: AsRef<Path>>(save_file: &SaveFile, api: &Api, paths: &[P]) -> Result<Infallible> {
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
                    // Send ReloadMessage using `api.send` instead of `api.reload`,
                    // because waiting for an answer would block the thread since the TCP socket is already in use.
                    api.send(MessageReload::new(json!([])).as_message())?;

                    // Add the paths as a component tag, so that reloaded paths will show up as tags.
                    // Then update the save file.
                    for path in paths {
                        if let Ok(tag) = Tag::try_from(path.as_ref()) {
                            let mut save_file = SaveFile::read_from_path(&save_file.path)?;
                            if save_file.save.push_object_tag(tag) {
                                save_file.write()?;
                            }
                        }
                    }
                }
            }
            Err(err) => error!("{}", err),
        }
    }
}
