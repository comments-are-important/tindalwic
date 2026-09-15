#![allow(missing_docs)]

use std::io::prelude::*;
use std::path::PathBuf;
use std::{fs, io};

use anyhow::{Error, Result, bail};
use bumpalo::Bump;
use clap::{Parser, Subcommand, ValueEnum};
use tindalwic::bumpalo::Arena;
use tindalwic_cli::*;
use tindalwic_serde::Neutered;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Input {
    YAML,
    TOML,
    JSON,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Output {
    YAML,
    TOML,
    JSON,
    SCM, // https://tree-sitter.github.io/tree-sitter/cli/parse.html#--no-ranges
}

#[derive(Parser)]
#[command(version = tindalwic::VERSION)]
#[command(about = "Text In Nested Dictionaries And Lists With Important Comments")]
#[command(after_help = "Default FORMAT is tindalwic.")]
#[command(arg_required_else_help = true)]
struct Cli {
    /// default is to read input, subcommand generates instead of reading
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, value_name = "FORMAT")]
    read: Option<Input>,
    #[arg(short, long, value_name = "FORMAT")]
    write: Option<Output>,

    /// Instead of stdin  (--read overrides any .ext)
    #[arg(short, long, value_name = "FILE")]
    input: Option<PathBuf>,
    /// Instead of stdout (--write overrides any .ext)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a file with random structure and values (no input)
    Random(random::Args),
    /// Error if re-encoding differs from input (no output)
    Check,
}

impl Cli {
    fn normalize() -> Result<Self> {
        let mut cli = Cli::parse();
        if cli.read.is_none() {
            if let Some(input) = &cli.input {
                cli.read = if let Some(ext) = input.extension() {
                    match ext.as_encoded_bytes() {
                        b"tindalwic" | b"TINDALWIC" => None,
                        b"yml" | b"YML" => Some(Input::YAML),
                        b"yaml" | b"YAML" => Some(Input::YAML),
                        b"toml" | b"TOML" => Some(Input::TOML),
                        b"json" | b"JSON" => Some(Input::JSON),
                        _ => bail!("unknown .{} extension", ext.to_string_lossy()),
                    }
                } else {
                    bail!("need --read when --input lacks .ext")
                }
            }
        }
        if cli.write.is_none() {
            if let Some(output) = &cli.output {
                cli.write = if let Some(ext) = output.extension() {
                    match ext.as_encoded_bytes() {
                        b"tindalwic" | b"TINDALWIC" => None,
                        b"yml" | b"YML" => Some(Output::YAML),
                        b"yaml" | b"YAML" => Some(Output::YAML),
                        b"toml" | b"TOML" => Some(Output::TOML),
                        b"json" | b"JSON" => Some(Output::JSON),
                        b"scm" | b"SCM" => Some(Output::SCM),
                        _ => bail!("unknown .{} extension", ext.to_string_lossy()),
                    }
                } else {
                    bail!("need --write when --output lacks .ext")
                }
            }
        }
        match &cli.command {
            Some(Command::Random(_)) => {
                if cli.input.is_some() || cli.read.is_some() {
                    bail!("random precludes: --input, --read");
                }
            }
            Some(Command::Check) => {
                if cli.output.is_some() || cli.write.is_some() {
                    bail!("check precludes: --output, --write");
                }
                cli.write = match &cli.read {
                    None => None,
                    Some(Input::YAML) => Some(Output::YAML),
                    Some(Input::TOML) => Some(Output::TOML),
                    Some(Input::JSON) => Some(Output::JSON),
                };
            }
            _ => {}
        }
        Ok(cli)
    }
}

fn main() -> Result<()> {
    let cli = Cli::normalize()?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let mut content = String::new();
    let file = match &cli.command {
        Some(Command::Random(generate)) => generate.file(&mut arena)?,
        _ => {
            let path = if let Some(input) = &cli.input {
                content = fs::read_to_string(input)?;
                arena.intern(&input.to_string_lossy())
            } else {
                io::stdin().read_to_string(&mut content)?;
                "<stdin>"
            };
            match &cli.read {
                None => arena
                    .format_errors(path, &content, usize::MAX)
                    .map_err(Error::msg)?,
                Some(Input::YAML) => from_yaml(&content, Neutered::seed(&mut arena))?,
                Some(Input::TOML) => from_toml(&content, Neutered::seed(&mut arena))?,
                Some(Input::JSON) => from_json(&content, Neutered::seed(&mut arena))?,
            }
        }
    };
    let encoded = match &cli.write {
        None => file.to_string(),
        Some(Output::YAML) => into_yaml(&Neutered(file))?,
        Some(Output::TOML) => into_toml(&Neutered(file))?,
        Some(Output::JSON) => into_json(&Neutered(file))?,
        Some(Output::SCM) => sitter::Sitter::to_scheme(&file)?,
    };
    match &cli.command {
        Some(Command::Check) => {
            if encoded != content {
                bail!("different")
            }
        }
        _ => {
            if let Some(output) = &cli.output {
                fs::write(output, encoded)?;
            } else {
                print!("{encoded}");
            }
        }
    }
    Ok(())
}
