use bumpalo::Bump;
use js_sys::{Array, Object, Reflect};
use tindalwic::File;
use tindalwic::alloc::Arena;
use tindalwic::serde::Neutered;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn initialize() {
    console_error_panic_hook::set_once();
}

fn output(tindalwic: bool, content: String) -> Result<JsValue, String> {
    let obj = Object::new();
    Reflect::set(&obj, &"tindalwic".into(), &tindalwic.into()).map_err(|e| format!("{:?}", e))?;
    Reflect::set(&obj, &"content".into(), &content.into()).map_err(|e| format!("{:?}", e))?;
    Ok(obj.into())
}

fn json_pretty(file: File) -> Result<JsValue, String> {
    serde_json::to_string_pretty(&Neutered(file))
        .map_or_else(|e| Err(e.to_string()), |c| output(false, c))
}
fn json_value(input: &str) -> Option<serde_json::Value> {
    serde_json::from_str(input).ok()
}

fn toml_pretty(file: File) -> Result<JsValue, String> {
    toml::to_string_pretty(&Neutered(file))
        .map_or_else(|e| Err(e.to_string()), |c| output(false, c))
}
fn toml_value(input: &str) -> Option<toml::Value> {
    toml::from_str(input).ok()
}

fn yaml_pretty(file: File) -> Result<JsValue, String> {
    yaml_serde::to_string(&Neutered(file)).map_or_else(|e| Err(e.to_string()), |c| output(false, c))
}
fn yaml_value(input: &str) -> Option<yaml_serde::Value> {
    yaml_serde::from_str(input).ok()
}

#[wasm_bindgen]
pub fn convert(input: String, hint: String, errors: Array) -> Result<JsValue, String> {
    let bump = Bump::new();
    let arena = Arena::new(&bump);
    let parsed = arena.parse(&input, |err| {
        errors.push(&JsValue::from_str(&format!("{:?}", err)));
        if errors.length() >= 10 {
            panic!("too many errors");
        }
    });
    if errors.length() == 0 {
        if let Some(file) = parsed {
            return match &hint[..] {
                "JSON" => json_pretty(file),
                "TOML" => toml_pretty(file),
                "YAML" => yaml_pretty(file),
                _ => Err(String::from("hint must be one of: JSON, TOML, YAML")),
            };
        }
    }
    if let Some(value) = json_value(&input) {
        return match Neutered::deserialize(&arena, &value) {
            Ok(out) => output(true, out.to_string()),
            Err(err) => Err(format!("JSON conversion failed: {err}")),
        };
    }
    if let Some(value) = toml_value(&input) {
        return match Neutered::deserialize(&arena, value) {
            Ok(out) => output(true, out.to_string()),
            Err(err) => Err(format!("TOML conversion failed: {err}")),
        };
    }
    if let Some(value) = yaml_value(&input) {
        return match Neutered::deserialize(&arena, &value) {
            Ok(out) => output(true, out.to_string()),
            Err(err) => Err(format!("YAML conversion failed: {err}")),
        };
    }
    Err("unable to parse input (errors pushed)".to_string())
}
