//! CLI helpers that might also be useful in other apps.

use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Error, Result, bail};
use bumpalo::Bump;
use serde::de::DeserializeSeed;
use time::UtcDateTime;
use time::format_description::well_known::Iso8601;
use time::format_description::well_known::iso8601::{Config, TimePrecision};
use tindalwic::File;
use tindalwic::bumpalo::Arena;
use tindalwic_serde::Neutered;

pub mod random;
#[cfg(debug_assertions)]
pub mod sitter;

const ISO8601_SHORT: Iso8601<
    {
        Config::DEFAULT
            .set_time_precision(TimePrecision::Second {
                decimal_digits: None,
            })
            .encode()
    },
> = Iso8601;

/// UTC in ISO 8601 without fractional seconds.
pub fn now() -> String {
    UtcDateTime::now()
        .format(&ISO8601_SHORT)
        .expect("trimming fractions should work")
}

/// a few widely used data formats that can convert via serde to/from tindalwic.
/// at least one lacks `to_writer_pretty`, so we stick to the string functions.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum SerDe {
    JSON,
    TOML,
    YAML,
}
impl SerDe {
    /// recognizes the common file extensions
    pub fn from(value: &[u8]) -> Option<Self> {
        match value {
            b"json" | b"JSON" => Some(SerDe::JSON),
            b"toml" | b"TOML" => Some(SerDe::TOML),
            b"yaml" | b"YAML" => Some(SerDe::YAML),
            b"yml" | b"YML" => Some(SerDe::YAML),
            _ => None,
        }
    }

    /// serialize [File] to a [Neutered] pretty string
    pub fn ser(&self, file: &File) -> Result<String> {
        Ok(match self {
            SerDe::JSON => serde_json::to_string_pretty(&Neutered(*file))?,
            SerDe::TOML => toml_edit::ser::to_string_pretty(&Neutered(*file))?,
            SerDe::YAML => yaml_serde::to_string(&Neutered(*file))?,
        })
    }

    /// deserialize from a [Neutered] string to [File]
    pub fn de<'de, 'a>(&self, arena: &mut Arena<'a>, input: &'de str) -> Result<File<'a>> {
        Ok(match self {
            SerDe::JSON => {
                let mut de = serde_json::Deserializer::from_str(input);
                Neutered::seed(arena).deserialize(&mut de)?
            }
            SerDe::TOML => {
                let de = toml_edit::de::Deserializer::parse(input)?;
                Neutered::seed(arena).deserialize(de)?
            }
            SerDe::YAML => {
                let de = yaml_serde::Deserializer::from_str(input);
                Neutered::seed(arena).deserialize(de)?
            }
        })
    }
}

/// a [BufReader] and the format expected from it
pub struct Reader {
    /// the buffered stream
    pub buffer: Box<dyn BufRead>,
    /// the serde format
    pub format: SerDe,
}
impl Reader {
    /// read from [io::stdin] using the given format
    pub fn stdin(format: SerDe) -> Self {
        Reader {
            buffer: Box::new(BufReader::new(io::stdin())),
            format,
        }
    }
    /// read from filesystem using the given format
    pub fn open<P: AsRef<Path>>(path: P, format: SerDe) -> Result<Self> {
        Ok(Reader {
            buffer: Box::new(BufReader::new(fs::File::open(path)?)),
            format,
        })
    }
    /// interpret a CLI argument.
    /// Err if unrecognized, naked extension means stdin, else open.
    pub fn parse(arg: &PathBuf) -> Result<Self> {
        if let Some(format) = SerDe::from(arg.as_os_str().as_encoded_bytes()) {
            return Ok(Reader::stdin(format));
        }
        if let Some(extension) = arg.extension() {
            if let Some(format) = SerDe::from(extension.as_encoded_bytes()) {
                return Reader::open(arg, format);
            }
        }
        bail!("unrecognized format/extension: {}", arg.to_string_lossy())
    }
    /// deserialize to tindalwic and write to stdout
    pub fn run(&mut self) -> Result<()> {
        let mut input = String::new();
        self.buffer.read_to_string(&mut input)?;
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let file = self.format.de(&mut arena, &input)?;
        io::stdout().write(file.to_string().as_bytes())?;
        Ok(())
    }
}

/// a [BufWriter] and the format it expects
pub struct Writer {
    /// the buffered stream
    pub buffer: Box<dyn Write>,
    /// the serde format
    pub format: SerDe,
}
impl Writer {
    /// write to [io::stdout] using the given format
    pub fn stdout(format: SerDe) -> Self {
        Writer {
            buffer: Box::new(BufWriter::new(io::stdout())),
            format,
        }
    }
    /// write to filesystem using the given format
    pub fn create<P: AsRef<Path>>(path: P, format: SerDe) -> Result<Self> {
        Ok(Writer {
            buffer: Box::new(BufWriter::new(fs::File::create(path)?)),
            format,
        })
    }
    /// interpret a CLI argument.
    /// Err if unrecognized, naked extension means stdout, else create.
    pub fn parse(arg: &PathBuf) -> Result<Self> {
        if let Some(format) = SerDe::from(arg.as_os_str().as_encoded_bytes()) {
            return Ok(Writer::stdout(format));
        }
        if let Some(extension) = arg.extension() {
            if let Some(format) = SerDe::from(extension.as_encoded_bytes()) {
                return Writer::create(arg, format);
            }
        }
        bail!("unrecognized format/extension: {}", arg.to_string_lossy())
    }
    /// read tindalwic from stdin, then serialize
    pub fn run(&mut self) -> Result<()> {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let parsed = arena.format_errors("<stdin>", &input, usize::MAX);
        let file = parsed.map_err(Error::msg)?;
        self.buffer.write(self.format.ser(&file)?.as_bytes())?;
        Ok(())
    }
}
