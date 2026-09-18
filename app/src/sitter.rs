//! the core crate is authoritative so the tree-sitter needs to conform.
//! a strategy for harmonizing is to generate the expected output for tests
//! mechanically then tweak the grammar code so it produces correct trees.
//!
//! this module is elided from release builds because it is a developer tool.

use anyhow::{Result, bail};
use bumpalo::Bump;
use std::path::PathBuf;
use tindalwic::{Comment, Entry, File, Item, Value, bumpalo::Arena};
use tree_sitter_cli::test::{TestEntry, parse_tests};

/// overwrite our tree-sitter tests
#[derive(Default)]
pub struct Corpus {
    active: bool,
    tests: Vec<TestTXT>,
    count: usize,
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
    fn visit(&mut self, test: TestEntry) -> Result<()> {
        match test {
            TestEntry::Group {
                name,
                children,
                file_path,
            } => {
                if self.active {
                    let Some(path) = self.tests.last().map(|it| &it.path) else {
                        bail!("found group '{name}' in a dangling test file");
                    };
                    bail!("found group '{name}' inside {:?}", path);
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
            TestEntry::Example { name, input, .. } => {
                self.count += 1;
                if !self.active {
                    bail!("found entry '{name}' but not active");
                };
                let Some(txt) = self.tests.last_mut() else {
                    bail!("found entry '{name}' without any TestTXT");
                };
                if txt.indent != 0 {
                    bail!("impossible: indent was not cleared");
                }
                let input = str::from_utf8(&input)?;
                let bump = Bump::new();
                let mut arena = Arena::new(&bump);
                let parsed = match arena.format_errors("", &input, usize::MAX) {
                    Err(error) => Err(error),
                    Ok(file) => {
                        let encoded = file.to_string();
                        if encoded == input {
                            Ok(file)
                        } else {
                            Err(encoded)
                        }
                    }
                };
                txt.result(self.count, &name[..], input, parsed);
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
        self.text("key", &entry.name.key);
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
                self.push("(dict");
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
    fn result(&mut self, num: usize, mut name: &str, input: &str, result: Result<File, String>) {
        self.pushln("===");
        // some test-sitter CLI commands have --test-number arg, but `test` can
        // only filter by regex on the name, so renumber...
        while name.starts_with(&['=', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9']) {
            name = &name[1..];
        }
        self.push(&num.to_string());
        self.push("=");
        if !name.starts_with(' ') {
            self.push(" ");
        }
        self.pushln(&name);
        if result.is_err() {
            self.pushln(":error");
        }
        self.pushln("===");
        if !input.is_empty() {
            self.pushln(&input);
        }
        self.pushln("---");
        match result {
            Ok(file) => self.file(&file),
            Err(msg) => self.push(&msg),
        }
        self.newline();
    }
}
