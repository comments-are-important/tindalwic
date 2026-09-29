//! this module is elided from release builds because it is a developer tool.

use anyhow::{Error, Result};
use bumpalo::Bump;
use std::io::{self, Read};
use tindalwic::{Comment, Entry, File, Item, Value, bumpalo::Arena};

/// the core crate is authoritative so the lezer needs to conform.
/// a strategy for harmonizing is to generate the expected output for tests
/// mechanically then tweak the grammar code so it produces correct trees.
#[derive(Default)]
pub struct Lezer {
    content: String,
    indent: usize,
}
impl Lezer {
    /// read tindalwic from stdin, write expected lezer tree
    pub fn run() -> Result<()> {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let parsed = arena.format_errors("<stdin>", &input, usize::MAX);
        let file = parsed.map_err(Error::msg)?;
        let mut corpus = Lezer {
            content: String::new(),
            indent: 0,
        };
        corpus.file(&file);
        print!("{}", corpus.content);
        Ok(())
    }
    fn push(&mut self, s: &str) {
        self.content.push_str(s);
    }
    fn pushln(&mut self, s: &str) {
        self.push(s);
        self.newline();
    }
    fn more(&mut self) {
        self.indent += 1;
    }
    fn less(&mut self) {
        self.indent -= 1;
    }
    fn newline(&mut self) {
        self.content.push('\n');
        for _ in 0..self.indent {
            self.content.push('\t');
        }
    }
    fn file(&mut self, file: &File) {
        self.push("(File");
        self.more();
        self.comment("Shebang", &file.hashbang);
        self.comment("Prolog", &file.prolog);
        for cell in file.cells {
            self.entry(&cell.get());
        }
        self.less();
        self.pushln(")");
    }
    fn entry(&mut self, entry: &Entry) {
        self.newline();
        self.push("(Entry");
        self.more();
        if entry.name.gap {
            self.newline();
            self.push("(Gap)");
        }
        self.comment("Comment", &entry.name.comment);
        self.text("key", &entry.name.key);
        self.item(&entry.item);
        self.push(")");
        self.less();
    }
    fn item(&mut self, item: &Item) {
        match item {
            Item::Text { value, epilog } => {
                self.text("Text", &value);
                self.comment("Epilog", epilog);
            }
            Item::List {
                prolog,
                cells,
                epilog,
            } => {
                self.comment("Prolog", prolog);
                self.newline();
                self.push("(List");
                self.more();
                for cell in *cells {
                    self.item(&cell.get());
                }
                self.push(")");
                self.less();
                self.comment("Epilog", epilog);
            }
            Item::Dict {
                prolog,
                cells,
                epilog,
            } => {
                self.comment("Prolog", prolog);
                self.newline();
                self.push("(Dict");
                self.more();
                for cell in *cells {
                    self.entry(&cell.get());
                }
                self.push(")");
                self.less();
                self.comment("Epilog", epilog);
            }
        }
    }
    fn comment(&mut self, tag: &str, maybe: &Option<Comment>) {
        if let Some(comment) = maybe {
            self.text(tag, &comment.value);
        }
    }
    fn text(&mut self, tag: &str, value: &Value) {
        self.newline();
        self.push("(");
        self.push(tag);
        self.more();
        for _ in value.lines() {
            self.newline();
            self.push("(Line)");
        }
        self.push(")");
        self.less();
    }
}
