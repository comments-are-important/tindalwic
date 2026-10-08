//! CLI helpers that might also be useful in other apps.

use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::PathBuf;

use anyhow::{Error, Result, bail};
use bumpalo::Bump;
use either::Either;
use serde::de::DeserializeSeed as _;
use time::UtcDateTime;
use time::format_description::well_known::Iso8601;
use time::format_description::well_known::iso8601::{Config, TimePrecision};
use tindalwic::File;
use tindalwic::bumpalo::Arena;
use tindalwic_serde::Neutered;

#[cfg(debug_assertions)]
pub mod lezer;
pub mod random;

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

/// fully read the given file, DASH means stdin (never returning if no EOF)
pub fn read_to_string(input: &PathBuf) -> Result<String> {
    let mut content = String::new();
    if input.as_os_str().as_encoded_bytes() == b"-" {
        io::stdin().lock().read_to_string(&mut content)?;
    } else {
        content = fs::read_to_string(input)?;
    }
    Ok(content)
}

/// a file opened for writing, or stdout
pub type Output<'a> = Either<io::BufWriter<fs::File>, io::StdoutLock<'a>>;
/// create the file for writing, DASH means stdout
pub fn create<'a>(output: &PathBuf) -> Result<Output<'a>> {
    Ok(if output.as_os_str().as_encoded_bytes() == b"-" {
        Either::Right(io::stdout().lock())
    } else {
        Either::Left(io::BufWriter::new(fs::File::create(output)?))
    })
}

/// verify a round trip does not change anything
pub fn idempotent(input: &PathBuf) -> Result<()> {
    let content = read_to_string(input)?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let parsed = arena.format_errors(&input.display(), &content, usize::MAX);
    let file = parsed.map_err(Error::msg)?;
    let encoded = file.to_string();
    if content != encoded {
        for diff in diff::lines(&content, &encoded) {
            match diff {
                diff::Result::Left(l) => eprintln!(" - {}", l),
                diff::Result::Both(l, _) => eprintln!("   {}", l),
                diff::Result::Right(r) => eprintln!(" + {}", r),
            }
        }
        bail!("different")
    }
    Ok(())
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
    /// recognizes the extension from the given path
    pub fn ext(path: &PathBuf) -> Option<Self> {
        path.extension()
            .and_then(|ext| SerDe::from(ext.as_encoded_bytes()))
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

/// deserialize input to tindalwic, encode and write to output
pub fn de(input: &PathBuf, output: &PathBuf) -> Result<()> {
    if input.as_os_str().as_encoded_bytes() == b"-" {
        bail!("dash for input is not allowed here, need the format JSON/TOML/YAML");
    }
    let stdin = SerDe::from(input.as_os_str().as_encoded_bytes());
    let or_ext = stdin.or_else(|| SerDe::ext(input));
    let Some(found) = or_ext else {
        bail!("need a format or an extension for input");
    };
    let dash = PathBuf::from("-");
    let content = read_to_string(if stdin.is_some() { &dash } else { input })?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let file = found.de(&mut arena, &content)?;
    create(output)?.write(file.to_string().as_bytes())?;
    Ok(())
}

/// parse tindalwic input, serialize to serde output
pub fn ser(input: &PathBuf, output: &PathBuf) -> Result<()> {
    if output.as_os_str().as_encoded_bytes() == b"-" {
        bail!("dash for output is not allowed here, need the format JSON/TOML/YAML");
    }
    let stdin = SerDe::from(output.as_os_str().as_encoded_bytes());
    let or_ext = stdin.or_else(|| SerDe::ext(output));
    let Some(found) = or_ext else {
        bail!("need a format or an extension for output");
    };
    let content = read_to_string(input)?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let parsed = arena.format_errors(&input.display(), &content, usize::MAX);
    let file = parsed.map_err(Error::msg)?;
    create(output)?.write(found.ser(&file)?.as_bytes())?;
    Ok(())
}
