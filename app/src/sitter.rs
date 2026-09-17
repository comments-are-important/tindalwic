#![allow(missing_docs)]

use anyhow::{Error, Result, bail};
use bumpalo::Bump;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use tindalwic::{Comment, Entry, File, Item, Value, bumpalo::Arena};
use tree_sitter_cli::test::{TestEntry, TestExpectation, parse_tests};

pub fn run() -> Result<()> {
    let path: PathBuf = ["grammar", "test", "corpus"].iter().collect();
    visit(parse_tests(&path)?, None)?;
    Ok(())
}
struct TestTXT {
    buf: String,
    path: String,
}
fn visit(test: TestEntry, parent: Option<&mut TestTXT>) -> Result<()> {
    match test {
        TestEntry::Group {
            name,
            children,
            file_path,
        } => {
            if let Some(parent) = parent {
                bail!("found group '{name}' inside {:?}", parent.path);
            }
            let file_path = file_path.unwrap_or(PathBuf::new());
            let ext = file_path.extension().unwrap_or(std::ffi::OsStr::new(""));
            if ext != "txt" {
                for child in children {
                    visit(child, None)?;
                }
            } else {
                let mut parent = TestTXT {
                    buf: String::new(),
                    path: String::from(file_path.to_string_lossy()),
                };
                for child in children {
                    visit(child, Some(&mut parent))?;
                }
                fs::write(file_path, parent.buf)?;
            }
            Ok(())
        }
        TestEntry::Example {
            name,
            input,
            output,
            attributes,
            attributes_str,
            header_delim_len,
            divider_delim_len,
            ..
        } => {
            let Some(parent) = parent else {
                bail!("found entry without a TestTXT as parent");
            };
            let input = str::from_utf8(&input)?;
            let output = if attributes.expectation != TestExpectation::Pass {
                output
            }else{
                let bump = Bump::new();
                let mut arena = Arena::new(&bump);
                let file = arena
                    .format_errors(&name, &input, usize::MAX)
                    .map_err(Error::msg)?;
                Sitter::to_scm(&file, true)
            };
            let header = "=".repeat(header_delim_len);
            let divider = "-".repeat(divider_delim_len);
            write!(
                parent.buf,
                "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
                header, name, attributes_str, header, input, divider, output
            )?;
            Ok(())
        }
    }
}

pub struct Sitter {
    buf: String,
    short: bool,
    indent: usize,
}
impl Sitter {
    fn more(&mut self) {
        self.indent += 2;
    }
    fn less(&mut self) {
        self.indent -= 2;
    }
    fn newline(&mut self) {
        if self.short {
            self.buf.push(' ');
        } else {
            self.buf.push('\n');
            for _ in 0..self.indent {
                self.buf.push(' ');
            }
        }
    }
    pub fn to_scm(file: &File, short: bool) -> String {
        let mut sitter = Sitter {
            buf: String::new(),
            short,
            indent: 0,
        };
        sitter.file(file);
        sitter.buf
    }
    fn file(&mut self, file: &File) {
        self.buf.push_str("(file");
        self.more();
        self.comment("shebang", &file.hashbang);
        self.comment("prolog", &file.prolog);
        for cell in file.cells {
            self.entry(&cell.get());
        }
        self.buf.push(')');
        self.less();
    }
    fn entry(&mut self, entry: &Entry) {
        self.newline();
        self.buf.push_str("(entry");
        self.more();
        if entry.name.gap {
            self.newline();
            self.buf.push_str("(gap)");
        }
        self.comment("comment", &entry.name.comment);
        self.item(&entry.item);
        self.buf.push(')');
        self.less();
    }
    fn item(&mut self, item: &Item) {
        match item {
            Item::Text { value, epilog } => {
                self.text("text", &value);
                self.comment("epilog", epilog);
            }
            Item::List {
                prolog,
                cells,
                epilog,
            } => {
                self.comment("prolog", prolog);
                self.newline();
                self.buf.push_str("(list");
                self.more();
                for cell in *cells {
                    self.item(&cell.get());
                }
                self.buf.push(')');
                self.less();
                self.comment("epilog", epilog);
            }
            Item::Dict {
                prolog,
                cells,
                epilog,
            } => {
                self.comment("prolog", prolog);
                self.newline();
                self.buf.push_str("(list");
                self.more();
                for cell in *cells {
                    self.entry(&cell.get());
                }
                self.buf.push(')');
                self.less();
                self.comment("epilog", epilog);
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
        self.buf.push('(');
        self.buf.push_str(tag);
        self.more();
        for _ in value.lines() {
            self.newline();
            self.buf.push_str("(line)");
        }
        self.buf.push(')');
        self.less();
    }
}
