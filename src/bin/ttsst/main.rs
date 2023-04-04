mod app;
mod macros;

use clap::{Parser, Subcommand};
use colorize::AnsiColor;
use std::path::PathBuf;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Attach a script to one or more objects
    Attach {
        /// Path to the file that should be attached
        #[arg(value_parser)]
        path: PathBuf,

        /// Optional: The guid(s) of the object(s) the script should be attached to.
        ///
        /// It is possible to attach a script to multiple objects at once.
        /// If guids are not provided a selection prompt will be shown.
        #[arg(value_parser)]
        guids: Option<Vec<String>>,
    },

    /// Detach a script from one or more objects
    Detach {
        /// Optional: The guid(s) of the object(s) the script should be detached from.
        ///
        /// It is possible to detach a script from multiple objects at once.
        /// If guids are not provided a selection prompt will be shown.
        #[arg(value_parser)]
        guids: Option<Vec<String>>,
    },

    /// Update scripts and reload save
    Reload {
        /// Path to the directory with all scripts
        #[arg(value_parser)]
        path: PathBuf,
    },

    /// Backup current save
    Backup {
        /// Path to save location
        #[arg(value_parser)]
        path: PathBuf,
    },
}

fn main() {
    let args = Args::parse();

    if let Err(err) = run(args) {
        eprintln!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let api = ExternalEditorApi::new();
    match args.command {
        Commands::Attach { path, guids } => app::attach(&api, &path, guids)?,
        Commands::Detach { guids } => app::detach(&api, guids)?,
        Commands::Backup { path } => app::backup(&api, &path)?,
        Commands::Reload { path } => app::reload(&api, &path)?,
    }
    Ok(())
}
