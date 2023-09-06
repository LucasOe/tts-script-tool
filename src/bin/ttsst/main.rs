mod app;
mod console;
mod logger;
mod msg;
mod parser;

use crate::logger::ConsoleLogger;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
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
        paths: Vec<PathBuf>,
    },

    /// Show print, log and error messages in the console
    Console,

    /// Watch file(s)
    Watch {
        #[arg(value_name = "PATH")]
        #[arg(value_parser = parser::path_exists, default_value = ".\\")]
        paths: Option<Vec<PathBuf>>,
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
        Commands::Reload { paths } => app::reload(&api, paths)?,
        Commands::Backup { path } => app::backup(&api, path)?,
        Commands::Console => console::start(api, None)?,
        Commands::Watch { paths } => console::start(api, paths)?,
    }
    Ok(())
}
