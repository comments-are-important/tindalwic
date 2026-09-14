use std::fmt::{self, Write as _};
use tindalwic::{Comment, Entry, File, Item, Value};

pub struct Sitter {
    buf: String,
}
impl fmt::Write for Sitter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }
}
impl Sitter {
    pub fn to_scheme(file: &File) -> anyhow::Result<String> {
        let mut sitter = Sitter { buf: String::new() };
        sitter.file(file)?;
        Ok(sitter.buf)
    }
    fn file(&mut self, file: &File) -> anyhow::Result<()> {
        self.write_str("(file")?;
        self.comment(2, "shebang", &file.hashbang)?;
        self.comment(2, "prolog", &file.prolog)?;
        for cell in file.cells {
            self.entry(2, &cell.get())?;
        }
        self.write_str(")\n")?;
        Ok(())
    }
    fn entry(&mut self, mut indent: usize, entry: &Entry) -> anyhow::Result<()> {
        write!(self, "\n{:indent$}(entry", "")?;
        indent += 2;
        if entry.gap {
            write!(self, "\n{:indent$}(gap)", "")?;
        }
        self.comment(indent, "before", &entry.before)?;
        self.item(indent, &entry.item)?;
        write!(self, ")")?;
        Ok(())
    }
    fn item(&mut self, indent: usize, item: &Item) -> anyhow::Result<()> {
        match item {
            Item::Text { value, epilog } => {
                self.text(indent, "text", &value)?;
                self.comment(indent, "epilog", epilog)?;
            }
            Item::List {
                prolog,
                cells: _,
                epilog,
            } => {
                self.comment(indent, "prolog", prolog)?;

                self.comment(indent, "epilog", epilog)?;
            }
            Item::Dict {
                prolog,
                cells: _,
                epilog,
            } => {
                self.comment(indent, "prolog", prolog)?;

                self.comment(indent, "epilog", epilog)?;
            }
        }
        Ok(())
    }
    fn comment(
        &mut self,
        indent: usize,
        tag: &str,
        maybe: &Option<Comment>,
    ) -> anyhow::Result<()> {
        if let Some(comment) = maybe {
            self.text(indent, tag, &comment.value)?;
        };
        Ok(())
    }
    fn text(&mut self, mut indent: usize, tag: &str, value: &Value) -> anyhow::Result<()> {
        write!(self, "\n{:indent$}({tag}", "")?;
        indent += 2;
        for _ in value.lines() {
            write!(self, "\n{:indent$}(line)", "")?;
        }
        self.write_str(")")?;
        Ok(())
    }
}
