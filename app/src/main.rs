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
        /// path with known extension, OR format (for stdout)
        output: PathBuf,
        /// tindalwic file to read (use `-` for stdin)
        input: PathBuf,
    },
    /// Deserialize from JSON/TOML/YAML
    From {
        /// path with known extension, OR format (for stdin)
        input: PathBuf,
        /// tindalwic file to write (use `-` for stdout)
        output: PathBuf,
    },
    /// Parse, encode, then fail if changed
    Same {
        /// tindalwic file to read (use `-` for stdin)
        input: PathBuf,
    },
    /// Generate a file with random structure and values
    #[command(alias = "rand")]
    Random {
        #[command(flatten)]
        args: random::Args,
    },
    /// Write expected Lezer tree
    #[cfg(debug_assertions)]
    Lezer {
        /// tindalwic file to read (use `-` for stdin)
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::To { output, input } => ser(input, output),
        Command::From { input, output } => de(input, output),
        Command::Same { input } => idempotent(input),
        Command::Random { args } => args.run(),
        #[cfg(debug_assertions)]
        Command::Lezer { input } => lezer::run(input),
    }
}
