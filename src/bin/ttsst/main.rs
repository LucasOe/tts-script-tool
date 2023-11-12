mod app;
mod console;
mod logger;
mod parser;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::logger::ConsoleLogger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level (use up to 2 times for more detailed output)
    #[arg(short = 'v', long = "verbose", global = true)]
    #[arg(action = clap::ArgAction::Count)]
    pub verbosity: u8,
}

#[derive(Args, Debug)]
pub struct Guids {
    /// Optional: The GUID(s) of the object(s) the Lua script or XML UI should be attached to
    #[arg(value_name = "GUID(s)")]
    #[arg(value_parser = parser::guid)]
    guids: Option<Vec<String>>,

    /// Show hidden objects like Zones in the selection prompt, if no GUIDs are provided
    #[arg(short, long)]
    all: bool,
}

#[derive(Args, Debug)]
pub struct ReloadArgs {
    /// Reload a single object
    #[arg(short, long, value_name = "GUID")]
    #[arg(value_parser = parser::guid)]
    guid: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Attach Lua scripts or XML UI to object(s)
    Attach {
        /// Path to the Lua script or XML UI that should be attached
        #[arg(value_name = "FILE")]
        #[arg(value_parser = parser::path_is_file)]
        path: PathBuf,

        #[command(flatten)]
        guids: Guids,
    },

    /// Detach Lua scripts and XML UI from object(s)
    Detach {
        #[command(flatten)]
        guids: Guids,
    },

    /// Reload script path(s)
    Reload {
        /// The script path(s) to reload
        #[arg(value_name = "PATH(S)")]
        #[arg(value_parser = parser::path_exists, default_value = ".\\")]
        paths: Vec<PathBuf>,

        #[command(flatten)]
        args: ReloadArgs,
    },

    /// Mirror Tabletop Simulator messages to the console
    Console,

    /// Watch script path(s) and reload on change
    Watch {
        /// The path(s) that will be watched for changes
        #[arg(value_name = "PATH(S)")]
        #[arg(value_parser = parser::path_exists, default_value = ".\\")]
        paths: Vec<PathBuf>,
    },

    /// Create a backup of the current save as a JSON file
    Backup {
        /// Path to save location
        #[arg(value_parser = parser::path_is_json)]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        log::error!("{}", err);
        std::process::exit(1);
    }
}

fn run(args: Cli) -> Result<()> {
    use log::LevelFilter;
    ConsoleLogger::new().init(match args.verbosity {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    })?;

    let api = tts_external_api::ExternalEditorApi::new();
    match args.command {
        Commands::Attach { path, guids } => app::attach(&api, path, guids)?,
        Commands::Detach { guids } => app::detach(&api, guids)?,
        Commands::Reload { paths, args } => app::reload(&api, &paths, args)?,
        Commands::Console => console::start::<PathBuf>(&api, None),
        Commands::Watch { paths } => console::start(&api, Some(&paths)),
        Commands::Backup { path } => app::backup(&api, path)?,
    }
    Ok(())
}
