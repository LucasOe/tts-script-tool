mod app;
mod macros;

use app::{attach, backup, reload};
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
        /// If not provided a selection prompt will be shown.
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
        Commands::Attach { path, guids } => attach(&api, &path, guids)?,
        Commands::Backup { path } => backup(&api, &path)?,
        Commands::Reload { path } => reload(&api, &path)?,
    }
    Ok(())
}
