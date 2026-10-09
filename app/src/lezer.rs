//! this module is elided from release builds because it is a developer tool.

use anyhow::{Error, Result};
use bumpalo::Bump;
use std::path::PathBuf;
use tindalwic::bumpalo::Arena;
use tindalwic::{
    Comment, CommentMarker, Dict, Entries, Entry, File, Item, Items, List, Text, Value,
};

/// read tindalwic, print expected lezer tree to stdout
/// the core crate is authoritative so the lezer needs to conform.
/// a strategy for harmonizing is to generate the expected output for tests
/// mechanically then tweak the grammar code so it produces correct trees.
pub fn run(input: &PathBuf) -> Result<()> {
    let content = super::read_to_string(input)?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let parsed = arena.format_errors(&input.display(), &content, usize::MAX);
    println!("{}", file(&parsed.map_err(Error::msg)?));
    Ok(())
}

fn file(file: &File) -> String {
    let mut kids = Vec::new();
    if let Some(shebang) = &file.hashbang {
        kids.push(format!(
            "Shebang(\"{}\",{})",
            CommentMarker::Shebang.token(),
            lines(&shebang, Some("Interpreter"))
        ))
    }
    kids.extend(comment(CommentMarker::Prolog, &file.prolog.value));
    for kid in &file.entries {
        kids.push(entry(&kid))
    }
    if kids.is_empty() {
        format!("File")
    } else {
        format!("File({})", kids.join(","))
    }
}

fn entry(entry: &Entry) -> String {
    let mut parts = Vec::new();
    parts.extend(comment(CommentMarker::NameTag, &entry.name.comment.value));
    let (bra, ket) = match &entry.item {
        Item::Text(_) => ("\"<\"", "\">\""),
        Item::List(_) => ("\"[\"", "\"]\""),
        Item::Dict(_) => ("\"{\"", "\"}\""),
    };
    parts.push(bra.into());
    parts.push(text("Key", &entry.name.key));
    parts.push(ket.into());
    parts.push(item(&entry.item));
    parts.extend(epilog(&entry.item));
    format!("Entry({})", parts.join(","))
}

fn item(item: &Item) -> String {
    match item {
        Item::Text(text_) => text("Text", &text_.value),
        Item::List(list_) => list(&list_.prolog, &list_.items),
        Item::Dict(dict_) => dict(&dict_.prolog, &dict_.entries),
    }
}

fn dict(prolog: &Comment, entries: &Entries) -> String {
    let mut kids = Vec::new();
    kids.extend(comment(CommentMarker::Prolog, &prolog.value));
    for kid in entries {
        kids.push(entry(&kid))
    }
    if kids.is_empty() {
        format!("Dict")
    } else {
        format!("Dict({})", kids.join(","))
    }
}

fn list(prolog: &Comment, items: &Items) -> String {
    let mut kids = Vec::new();
    kids.extend(comment(CommentMarker::Prolog, &prolog.value));
    for kid in items {
        let mut parts = Vec::new();
        let bracket = match &kid {
            Item::Text(text_) => {
                parts.push(text("Text", &text_.value));
                "\"<>\""
            }
            Item::List(list_) => {
                parts.push(list(&list_.prolog, &list_.items));
                "\"[]\""
            }
            Item::Dict(dict_) => {
                parts.push(dict(&dict_.prolog, &dict_.entries));
                "\"{}\""
            }
        };
        parts.extend(epilog(&kid));
        kids.push(format!("Item({bracket},{})", parts.join(",")));
    }
    if kids.is_empty() {
        format!("List")
    } else {
        format!("List({})", kids.join(","))
    }
}

fn epilog(item: &Item) -> Option<String> {
    comment(
        CommentMarker::Epilog,
        match item {
            Item::Text(Text { epilog, .. }) => epilog,
            Item::List(List { epilog, .. }) => &epilog.value,
            Item::Dict(Dict { epilog, .. }) => &epilog.value,
        },
    )
}

fn comment(marker: CommentMarker, maybe: &Option<Value>) -> Option<String> {
    let Some(comment) = maybe else {
        return None;
    };
    let node = match marker {
        CommentMarker::Shebang => "Shebang",
        CommentMarker::Prolog => "Prolog",
        CommentMarker::Epilog => "Epilog",
        CommentMarker::NameTag => "Comment", // TODO harmonize grammar
    };
    return Some(format!(
        "{node}(\"{}\",{})",
        marker.token(),
        text("GFM", &comment)
    ));
}

fn text(tag: &str, value: &Value) -> String {
    format!("{tag}({})", lines(value, None))
}

fn lines(value: &Value, wrap_first: Option<&str>) -> String {
    let first = match wrap_first {
        None => "Line".to_owned(),
        Some(wrap) => format!("{wrap}(Line)"),
    };
    match value.lines().count() {
        0 => "".to_owned(),
        n => format!("{first}{}", ",Line".repeat(n - 1)),
    }
}
