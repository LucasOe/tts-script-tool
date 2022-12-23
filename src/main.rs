use anyhow::Result;
use clap::{Parser, Subcommand};
use colorize::AnsiColor;
use std::path::PathBuf;
use ttsst::{attach, backup, reload};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Attach script to object
    Attach {
        /// Path to the file that should be attached
        #[clap(parse(from_os_str))]
        path: PathBuf,
        /// Optional: The guid of the object the script should be attached to.
        /// If not provided a list of all objects will be shown.
        #[clap(value_parser)]
        guid: Option<String>,
    },
    /// Update scripts and reload save
    Reload {
        /// Path to the directory with all scripts
        #[clap(parse(from_os_str))]
        path: PathBuf,
    },
    /// Backup current save
    Backup {
        /// Path to save location
        #[clap(parse(from_os_str))]
        path: PathBuf,
    },
}

fn main() {
    let args = Args::parse();

    if let Err(err) = run(args) {
        println!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    match args.command {
        Commands::Attach { path, guid } => attach(&path, guid)?,
        Commands::Backup { path } => backup(&path)?,
        Commands::Reload { path } => reload(&path)?,
    }
    Ok(())
}
