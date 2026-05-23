use bumpalo::Bump;
use serde::Serialize;
use serde::de::DeserializeSeed;
use tindalwic::File;
use tindalwic::alloc::Arena;
use tindalwic::serde::{ArenaSeed, Compact, Neutered, Verbose};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn initialize() {
    console_error_panic_hook::set_once();
}

fn into_json<T: ?Sized + Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}
fn into_toml<T: ?Sized + Serialize>(value: &T) -> Result<String, String> {
    toml::to_string_pretty(value).map_err(|e| e.to_string())
}
fn into_yaml<T: ?Sized + Serialize>(value: &T) -> Result<String, String> {
    yaml_serde::to_string(value).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn from_tindalwic(
    input: String,  // the tindalwic data
    mode: String,   // Neutered | Compact | Verbose
    format: String, // JSON | TOML | YAML
) -> Result<String, String> {
    let bump = Bump::new();
    let arena = Arena::new(&bump);
    let mut error = false; // our codemirror lang provides feedback
    let parsed = arena.parse(&input, |_| {
        error = true;
    });
    if error {
        return Err("bad Tindalwic".to_string());
    }
    let Some(file) = parsed else {
        return Err("arena failure".to_string());
    };
    match (&mode[..], &format[..]) {
        ("Neutered", "JSON") => into_json(&Neutered(file)),
        ("Neutered", "TOML") => into_toml(&Neutered(file)),
        ("Neutered", "YAML") => into_yaml(&Neutered(file)),
        ("Compact", "JSON") => into_json(&Compact(file)),
        ("Compact", "TOML") => into_toml(&Compact(file)),
        ("Compact", "YAML") => into_yaml(&Compact(file)),
        ("Verbose", "JSON") => into_json(&Verbose(file)),
        ("Verbose", "TOML") => into_toml(&Verbose(file)),
        ("Verbose", "YAML") => into_yaml(&Verbose(file)),
        _ => Err("bad parameters".to_string()),
    }
}

fn from_json<'de, 'a>(
    input: &str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, String> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    seed.deserialize(value).map_err(|e| e.to_string())
}
fn from_toml<'de, 'a>(
    input: &str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, String> {
    let value: toml::Value = toml::from_str(input).map_err(|e| e.to_string())?;
    seed.deserialize(value).map_err(|e| e.to_string())
}
fn from_yaml<'de, 'a>(
    input: &str,
    seed: impl DeserializeSeed<'de, Value = File<'a>>,
) -> Result<File<'a>, String> {
    let value: yaml_serde::Value = yaml_serde::from_str(input).map_err(|e| e.to_string())?;
    seed.deserialize(value).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn into_tindalwic(
    input: String,  // the data in their format
    mode: String,   // Neutered | Compact | Verbose
    format: String, // JSON | TOML | YAML
) -> Result<String, String> {
    let bump = Bump::new();
    let arena = Arena::new(&bump);
    match (&mode[..], &format[..]) {
        ("Neutered", "JSON") => from_json(&input, Neutered::seed(&arena)),
        ("Neutered", "TOML") => from_toml(&input, Neutered::seed(&arena)),
        ("Neutered", "YAML") => from_yaml(&input, Neutered::seed(&arena)),
        ("Compact", "JSON") => from_json(&input, Compact::seed(&arena)),
        ("Compact", "TOML") => from_toml(&input, Compact::seed(&arena)),
        ("Compact", "YAML") => from_yaml(&input, Compact::seed(&arena)),
        ("Verbose", "JSON") => from_json(&input, Verbose::seed(&arena)),
        ("Verbose", "TOML") => from_toml(&input, Verbose::seed(&arena)),
        ("Verbose", "YAML") => from_yaml(&input, Verbose::seed(&arena)),
        _ => Err("bad parameters".to_string()),
    }
    .map(|f| f.to_string())
}
