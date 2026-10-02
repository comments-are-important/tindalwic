//! this module is elided from release builds because it is a developer tool.

use anyhow::{Error, Result};
use bumpalo::Bump;
use std::io::{self, Read};
use tindalwic::{Comment, Entries, Entry, File, Item, Items, Value, bumpalo::Arena};

/// read tindalwic from stdin, print expected lezer tree to stdout
/// the core crate is authoritative so the lezer needs to conform.
/// a strategy for harmonizing is to generate the expected output for tests
/// mechanically then tweak the grammar code so it produces correct trees.
pub fn run() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let bump = Bump::new();
    let mut arena = Arena::new(&bump);
    let parsed = arena.format_errors("<stdin>", &input, usize::MAX);
    println!("{}", file(&parsed.map_err(Error::msg)?));
    Ok(())
}

fn file(file: &File) -> String {
    let mut kids = Vec::new();
    kids.extend(comment("Shebang", &file.hashbang));
    kids.extend(comment("Prolog", &file.prolog));
    for kid in file.cells {
        kids.push(entry(&kid.get()))
    }
    if kids.is_empty() {
        format!("File")
    } else {
        format!("File({})", kids.join(","))
    }
}

fn entry(entry: &Entry) -> String {
    let mut parts = Vec::new();
    parts.extend(comment("Comment", &entry.name.comment));
    parts.push(text("Key", &entry.name.key));
    parts.push(item(&entry.item));
    parts.extend(epilog(&entry.item));
    format!("Entry({})", parts.join(","))
}

fn item(item: &Item) -> String {
    match item {
        Item::Text { value, .. } => text("Text", &value),
        Item::List { prolog, cells, .. } => list(prolog, cells),
        Item::Dict { prolog, cells, .. } => dict(prolog, cells),
    }
}

fn dict(prolog: &Option<Comment>, entries: Entries) -> String {
    let mut kids = Vec::new();
    kids.extend(comment("Prolog", prolog));
    for kid in entries {
        kids.push(entry(&kid.get()))
    }
    if kids.is_empty() {
        format!("Dict")
    } else {
        format!("Dict({})", kids.join(","))
    }
}

fn list(prolog: &Option<Comment>, items: Items) -> String {
    let mut kids = Vec::new();
    kids.extend(comment("Prolog", prolog));
    for kid in items {
        let item = kid.get();
        let mut parts = Vec::new();
        parts.push(match item {
            Item::Text { value, .. } => text("Text", &value),
            Item::List { prolog, cells, .. } => list(&prolog, cells),
            Item::Dict { prolog, cells, .. } => dict(&prolog, cells),
        });
        parts.extend(epilog(&item));
        kids.push(format!("Item({})", parts.join(",")));
    }
    if kids.is_empty() {
        format!("List")
    } else {
        format!("List({})", kids.join(","))
    }
}

fn epilog(item: &Item) -> Option<String> {
    comment(
        "Epilog",
        match item {
            Item::Text { epilog, .. } => epilog,
            Item::List { epilog, .. } => epilog,
            Item::Dict { epilog, .. } => epilog,
        },
    )
}

fn comment(tag: &str, maybe: &Option<Comment>) -> Option<String> {
    let Some(comment) = maybe else { return None };
    if tag != "Shebang" {
        return Some(format!("{tag}({})", text("GFM", &comment.value)));
    }
    Some(format!(
        "Shebang{}",
        lines(&comment.value, Some("Interpreter"))
    ))
}

fn text(tag: &str, value: &Value) -> String {
    format!("{tag}{}", lines(value, None))
}

fn lines(value: &Value, wrap_first: Option<&str>) -> String {
    let first = match wrap_first {
        None => "Line".to_owned(),
        Some(wrap) => format!("{wrap}(Line)"),
    };
    match value.lines().count() {
        0 => "".to_owned(),
        n => format!("({first}{})", ",Line".repeat(n - 1)),
    }
}
