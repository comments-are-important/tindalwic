//! the core crate is authoritative so the tree-sitter needs to conform.
//! a strategy for harmonizing is to generate the expected output for tests
//! mechanically then tweak the grammar code so it produces correct trees.
//!
//! this module is elided from release builds because it is a developer tool.

use anyhow::{Error, Result, bail};
use bumpalo::Bump;
use std::path::PathBuf;
use tindalwic::{Comment, Entry, File, Item, Value, bumpalo::Arena};
use tree_sitter_cli::test::{TestEntry, TestExpectation, parse_tests};

/// overwrite our tree-sitter tests
#[derive(Default)]
pub struct Corpus {
    active: bool,
    tests: Vec<TestTXT>,
}
impl Corpus {
    /// assumes PWD is root of repo
    pub fn edit() -> Result<()> {
        let mut corpus = Corpus::default();
        let path: PathBuf = ["grammar", "test", "corpus"].iter().collect();
        corpus.visit(parse_tests(&path)?)?;
        if corpus.active {
            bail!("impossible: active was not cleared");
        }
        for test in corpus.tests {
            std::fs::write(test.path, test.content)?;
        }
        Ok(())
    }
    fn active(&mut self) -> Result<Option<&mut TestTXT>> {
        if !self.active {
            Ok(None)
        } else if let Some(last) = self.tests.last_mut() {
            Ok(Some(last))
        } else {
            bail!("impossible: active while empty")
        }
    }
    fn visit(&mut self, test: TestEntry) -> Result<()> {
        match test {
            TestEntry::Group {
                name,
                children,
                file_path,
            } => {
                if let Some(txt) = self.active()? {
                    bail!("found group '{name}' inside {:?}", txt.path);
                }
                let file_path = file_path.unwrap_or(PathBuf::new());
                let ext = file_path.extension().unwrap_or(std::ffi::OsStr::new(""));
                if ext == "txt" {
                    self.tests.push(TestTXT {
                        path: file_path,
                        content: String::new(),
                        indent: 0,
                    });
                    self.active = true;
                }
                for child in children {
                    self.visit(child)?;
                }
                self.active = false;
                Ok(())
            }
            TestEntry::Example {
                name,
                input,
                output,
                attributes,
                attributes_str,
                ..
            } => {
                let Some(txt) = self.active()? else {
                    bail!("found entry '{name}' without active TestTXT");
                };
                if txt.indent != 0 {
                    bail!("impossible: indent was not cleared");
                }
                txt.pushln("===");
                txt.pushln(&name);
                // their lib can parse str->TestAttributes but can't go the other
                // direction, easiest to remove `:cst` attr by string manipulation...
                let attrs = (attributes_str + "\n").replace(":cst\n", "");
                if attrs.len() > 1 {
                    txt.push(&attrs);
                }
                txt.pushln("===");
                let input = str::from_utf8(&input)?;
                if !input.is_empty() {
                    txt.pushln(&input);
                }
                txt.pushln("---");
                if attributes.expectation != TestExpectation::Pass {
                    txt.pushln(&output);
                } else {
                    let bump = Bump::new();
                    let mut arena = Arena::new(&bump);
                    let file = arena
                        .format_errors(&name, &input, usize::MAX)
                        .map_err(Error::msg)?;
                    txt.file(&file);
                }
                Ok(())
            }
        }
    }
}

struct TestTXT {
    path: PathBuf,
    content: String,
    indent: usize,
}
impl TestTXT {
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
        self.push("(file");
        self.more();
        self.comment("shebang", &file.hashbang);
        self.comment("prolog", &file.prolog);
        for cell in file.cells {
            self.entry(&cell.get());
        }
        self.less();
        self.pushln(")");
        self.newline();
    }
    fn entry(&mut self, entry: &Entry) {
        self.newline();
        self.push("(entry");
        self.more();
        if entry.name.gap {
            self.newline();
            self.push("(gap)");
        }
        self.comment("comment", &entry.name.comment);
        self.item(&entry.item);
        self.push(")");
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
                self.push("(list");
                self.more();
                for cell in *cells {
                    self.item(&cell.get());
                }
                self.push(")");
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
                self.push("(list");
                self.more();
                for cell in *cells {
                    self.entry(&cell.get());
                }
                self.push(")");
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
        self.push("(");
        self.push(tag);
        self.more();
        for _ in value.lines() {
            self.newline();
            self.push("(line)");
        }
        self.push(")");
        self.less();
    }
}
