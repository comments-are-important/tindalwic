//! the clap parser definition and main function for the CLI.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tindalwic_cli::*;

#[derive(Parser)]
#[command(version = tindalwic::VERSION)]
#[command(about = "Text In Nested Dictionaries And Lists With Important Comments")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Serialize to JSON/TOML/YAML
    To {
        /// path with extension, or name of format to use stdout
        file_or_format: PathBuf,
    },
    /// Deserialize from JSON/TOML/YAML
    From {
        /// path with extension, or name of format to use stdin
        file_or_format: PathBuf,
    },
    /// Generate a file with random structure and values
    Random {
        #[command(flatten)]
        args: random::Args,
    },
    /// Edit expected results in Tree-sitter test corpus
    #[cfg(debug_assertions)]
    Sitter,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::To { file_or_format } => Writer::parse(file_or_format)?.run(),
        Command::From { file_or_format } => Reader::parse(file_or_format)?.run(),
        Command::Random { args } => args.run(),
        #[cfg(debug_assertions)]
        Command::Sitter => sitter::Corpus::edit(),
    }
}
