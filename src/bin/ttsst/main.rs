mod app;
mod console;
mod msg;
mod parser;

use clap::{Args, Parser, Subcommand};
use log::*;
use simplelog::{Color, ColorChoice, LevelFilter, TermLogger, TerminalMode};
use std::path::PathBuf;
use tts_external_api::ExternalEditorApi;
use ttsst::error::Result;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level - specify up to 2 times to get more detailed output.
    #[clap(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbosity: u8,
}

#[derive(Args, Debug)]
pub struct Guids {
    /// Optional: The guid(s) of the object(s) the script should be attached to
    #[arg(value_parser = parser::guid)]
    guids: Option<Vec<String>>,

    /// Show HandTriggers in the list of objects, if no guids are provided
    #[arg(short, long)]
    all: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Attach a script to one or more objects
    Attach {
        /// Path to the file that should be attached.
        /// ttsst will use the current working directory as a root.
        #[arg(value_name = "FILE")]
        #[arg(value_parser = parser::path_is_file)]
        path: PathBuf,

        #[command(flatten)]
        guids: Guids,
    },

    /// Detach a script from one or more objects
    Detach {
        #[command(flatten)]
        guids: Guids,
    },

    /// Update scripts and reload save
    Reload {
        /// Path to the directory with all scripts.
        ///
        /// If the path is a single file, only objects with that file attached will get reloaded.
        #[arg(value_name = "PATH")]
        #[arg(value_parser = parser::path_exists, default_value = ".\\")]
        path: PathBuf,
    },

    /// Show print, log and error messages in the console
    Console {
        /// Optional: Directory to be watched
        #[arg(short, long, value_name = "PATH")]
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
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        error!("{}", err);
        std::process::exit(1);
    }
}

fn init_logger(verbosity: u8) -> Result<()> {
    let log_level = match verbosity {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let config = simplelog::ConfigBuilder::new()
        .set_time_level(LevelFilter::Off)
        .set_level_color(Level::Error, Some(Color::Red))
        .set_level_color(Level::Warn, Some(Color::Yellow))
        .set_level_color(Level::Info, Some(Color::Green))
        .set_level_color(Level::Debug, Some(Color::Blue))
        .set_level_color(Level::Trace, Some(Color::Magenta))
        .build();

    TermLogger::init(log_level, config, TerminalMode::Mixed, ColorChoice::Auto)
        .map_err(|err| err.into())
}

fn run(args: Cli) -> Result<()> {
    init_logger(args.verbosity)?;

    let api = ExternalEditorApi::new();
    match args.command {
        Commands::Attach { path, guids } => app::attach(&api, path, guids)?,
        Commands::Detach { guids } => app::detach(&api, guids)?,
        Commands::Backup { path } => app::backup(&api, path)?,
        Commands::Console { watch } => console::start(api, watch)?,
        Commands::Reload { path } => app::reload(&api, path)?,
    }
    Ok(())
}
