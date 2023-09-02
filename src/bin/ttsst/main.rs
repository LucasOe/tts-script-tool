mod app;
mod console;
mod macros;
mod parser;

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
        #[arg(value_parser = parser::path_is_file)]
        path: PathBuf,

        /// Optional: The guid(s) of the object(s) the script should be attached to
        #[arg(value_parser = parser::guid)]
        guids: Option<Vec<String>>,

        /// Show HandTriggers in the list of objects, if no guids are provided
        #[arg(short, long)]
        all: bool,
    },

    /// Detach a script from one or more objects
    Detach {
        /// Optional: The guid(s) of the object(s) the script should be detached from
        #[arg(value_parser = parser::guid)]
        guids: Option<Vec<String>>,

        /// Show HandTriggers in the list of objects, if no guids are provided
        #[arg(short, long)]
        all: bool,
    },

    /// Update scripts and reload save
    Reload {
        /// Path to the directory with all scripts
        #[arg(value_parser = parser::path_exists, default_value = ".\\")]
        path: PathBuf,
    },

    /// Show print, log and error messages in the console
    Console {
        /// Optional: Directory to be watched
        #[arg(short, long)]
        #[arg(value_parser = parser::path_exists)]
        watch: Option<PathBuf>,
    },

    /// Backup current save
    Backup {
        /// Path to save location
        #[arg(value_parser = parser::path_is_json)]
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
        Commands::Attach { path, guids, all } => app::attach(&api, path, guids, all)?,
        Commands::Detach { guids, all } => app::detach(&api, guids, all)?,
        Commands::Backup { path } => app::backup(&api, path)?,
        Commands::Console { watch } => console::start(api, watch)?,
        Commands::Reload { path } => app::reload(&api, path)?,
    }
    Ok(())
}
