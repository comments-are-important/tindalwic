use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeSeed;
use time::UtcDateTime;
use time::format_description::well_known::Iso8601;
use time::format_description::well_known::iso8601::{Config, TimePrecision};
use tindalwic::File;

pub mod random;
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
pub fn now() -> String {
    UtcDateTime::now()
        .format(&ISO8601_SHORT)
        .expect("trimming fractions should work")
}

pub fn from_json<'de, 'a>(
    input: &'de str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, serde_json::Error> {
    let mut de = serde_json::Deserializer::from_str(input);
    seed.deserialize(&mut de)
}
pub fn from_toml<'de, 'a>(
    input: &'de str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, toml_edit::de::Error> {
    let de = toml_edit::de::Deserializer::parse(input)?;
    seed.deserialize(de)
}
pub fn from_yaml<'de, 'a>(
    input: &'de str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, yaml_serde::Error> {
    let de = yaml_serde::Deserializer::from_str(input);
    seed.deserialize(de)
}

pub fn into_json<T: ?Sized + Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}
pub fn into_toml<T: ?Sized + Serialize>(value: &T) -> Result<String, toml_edit::ser::Error> {
    toml_edit::ser::to_string_pretty(value)
}
pub fn into_yaml<T: ?Sized + Serialize>(value: &T) -> Result<String, yaml_serde::Error> {
    yaml_serde::to_string(value)
}
